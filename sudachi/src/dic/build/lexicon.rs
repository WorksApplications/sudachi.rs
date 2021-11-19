/*
 *  Copyright (c) 2021 Works Applications Co., Ltd.
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

use std::borrow::{Borrow, Cow};
use std::fmt::{Debug, Formatter};
use std::fs::File;
use std::io::Write;
use std::path::Path;

use csv::{StringRecord, Trim};
use indexmap::map::IndexMap;
use indexmap::Equivalent;
use memmap2::Mmap;

use crate::analysis::Mode;
use crate::dic::build::error::{BuildFailure, DicCompilationCtx, DicWriteResult};
use crate::dic::build::parse::{
    it_next, none_if_equal, parse_dic_form, parse_i16, parse_mode, parse_slash_list,
    parse_u32_list, parse_wordid, parse_wordid_list, unescape, unescape_cow, WORD_ID_LITERAL,
};
use crate::dic::build::primitives::{write_u32_array, Utf16Writer};
use crate::dic::build::report::{ReportBuilder, Reporter};
use crate::dic::build::MAX_POS_IDS;
use crate::dic::grammar::Grammar;
use crate::dic::word_id::WordId;
use crate::dic::POS_DEPTH;
use crate::error::SudachiResult;

#[cfg(test)]
mod test;

#[cfg(test)]
mod wordinfo_test;

#[derive(Hash, Eq, PartialEq)]
pub struct StrPosEntry {
    data: [Cow<'static, str>; POS_DEPTH],
}

impl<'a> Borrow<[Cow<'a, str>; POS_DEPTH]> for StrPosEntry {
    fn borrow(&self) -> &[Cow<'a, str>; POS_DEPTH] {
        &self.data
    }
}

impl<'a> Equivalent<[Cow<'a, str>; POS_DEPTH]> for StrPosEntry {
    fn equivalent(&self, key: &[Cow<'_, str>; POS_DEPTH]) -> bool {
        self.data.eq(key)
    }
}

impl StrPosEntry {
    /// owning means 'static
    fn rewrap(data: Cow<str>) -> Cow<'static, str> {
        match data {
            Cow::Borrowed(b) => Cow::Owned(b.to_owned()),
            Cow::Owned(s) => Cow::Owned(s),
        }
    }

    pub fn new(data: [Cow<str>; POS_DEPTH]) -> Self {
        let [d1, d2, d3, d4, d5, d6] = data;
        let owned: [Cow<'static, str>; POS_DEPTH] = [
            Self::rewrap(d1),
            Self::rewrap(d2),
            Self::rewrap(d3),
            Self::rewrap(d4),
            Self::rewrap(d5),
            Self::rewrap(d6),
        ];
        Self { data: owned }
    }

    pub fn from_built_pos(data: &Vec<String>) -> Self {
        let mut iter = data.iter().map(|x| x.as_str());
        let p1 = Cow::Borrowed(iter.next().unwrap());
        let p2 = Cow::Borrowed(iter.next().unwrap());
        let p3 = Cow::Borrowed(iter.next().unwrap());
        let p4 = Cow::Borrowed(iter.next().unwrap());
        let p5 = Cow::Borrowed(iter.next().unwrap());
        let p6 = Cow::Borrowed(iter.next().unwrap());
        Self::new([p1, p2, p3, p4, p5, p6])
    }

    pub fn fields(&self) -> &[Cow<'static, str>; 6] {
        &self.data
    }
}

impl Debug for StrPosEntry {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{},{},{},{},{},{}",
            self.data[0], self.data[1], self.data[2], self.data[3], self.data[4], self.data[5]
        )
    }
}

#[derive(PartialEq, Eq, Debug)]
pub(crate) enum SplitUnit {
    Ref(WordId),
    Inline {
        surface: String,
        pos: u16,
        reading: Option<String>,
    },
}

impl SplitUnit {
    pub fn format(&self, lexicon: &LexiconReader) -> String {
        match self {
            SplitUnit::Ref(id) => id.as_raw().to_string(),
            SplitUnit::Inline {
                surface,
                pos,
                reading,
            } => format!(
                "{},{:?},{}",
                surface,
                lexicon.pos_obj(*pos).unwrap(),
                reading.as_ref().unwrap_or(surface)
            ),
        }
    }
}

pub(crate) trait SplitUnitResolver {
    fn resolve(&self, unit: &SplitUnit) -> Option<WordId> {
        match unit {
            SplitUnit::Ref(wid) => Some(*wid),
            SplitUnit::Inline {
                surface,
                pos,
                reading,
            } => self.resolve_inline(&surface, *pos, reading.as_deref()),
        }
    }

    fn resolve_inline(&self, surface: &str, pos: u16, reading: Option<&str>) -> Option<WordId>;
}

pub(crate) struct RawLexiconEntry {
    pub left_id: i16,
    pub right_id: i16,
    pub cost: i16,
    pub surface: String,
    pub headword: Option<String>,
    pub dic_form: WordId,
    pub norm_form: Option<String>,
    pub pos: u16,
    pub splits_a: Vec<SplitUnit>,
    pub splits_b: Vec<SplitUnit>,
    pub reading: Option<String>,
    #[allow(unused)]
    pub splitting: Mode,
    pub word_structure: Vec<WordId>,
    pub synonym_groups: Vec<u32>,
}

impl RawLexiconEntry {
    pub fn surface(&self) -> &str {
        &self.surface
    }

    pub fn headword(&self) -> &str {
        self.headword.as_deref().unwrap_or_else(|| self.surface())
    }

    pub fn norm_form(&self) -> &str {
        self.norm_form.as_deref().unwrap_or_else(|| self.headword())
    }

    pub fn reading(&self) -> &str {
        self.reading.as_deref().unwrap_or_else(|| self.headword())
    }

    pub fn should_index(&self) -> bool {
        self.left_id >= 0
    }

    pub fn write_params<W: Write>(&self, w: &mut W) -> DicWriteResult<usize> {
        w.write_all(&self.left_id.to_le_bytes())?;
        w.write_all(&self.right_id.to_le_bytes())?;
        w.write_all(&self.cost.to_le_bytes())?;
        Ok(6)
    }

    pub fn write_word_info<W: Write>(
        &self,
        u16w: &mut Utf16Writer,
        w: &mut W,
    ) -> DicWriteResult<usize> {
        let mut size = 0;

        size += u16w.write(w, &self.headword())?; // surface of WordInfo
        size += u16w.write_len(w, self.surface.len())?; // surface for trie
        w.write_all(&self.pos.to_le_bytes())?;
        size += 2;
        size += u16w.write_empty_if_equal(w, self.norm_form(), self.headword())?;
        w.write_all(&self.dic_form.as_raw().to_le_bytes())?;
        size += 4;
        size += u16w.write_empty_if_equal(w, self.reading(), self.headword())?;
        size += write_u32_array(w, &self.splits_a)?;
        size += write_u32_array(w, &self.splits_b)?;
        size += write_u32_array(w, &self.word_structure)?;
        size += write_u32_array(w, &self.synonym_groups)?;

        Ok(size)
    }
}

pub struct LexiconReader {
    pos: IndexMap<StrPosEntry, u16>,
    ctx: DicCompilationCtx,
    entries: Vec<RawLexiconEntry>,
    unresolved: usize,
    start_pos: usize,
    max_left: i16,
    max_right: i16,
    num_system: usize,
}

impl LexiconReader {
    pub fn new() -> Self {
        Self {
            pos: IndexMap::new(),
            ctx: DicCompilationCtx::default(),
            entries: Vec::new(),
            unresolved: 0,
            start_pos: 0,
            max_left: i16::MAX,
            max_right: i16::MAX,
            num_system: usize::MAX,
        }
    }

    pub(crate) fn entries(&self) -> &[RawLexiconEntry] {
        &self.entries
    }

    pub fn needs_split_resolution(&self) -> bool {
        self.unresolved > 0
    }

    pub fn set_max_conn_sizes(&mut self, left: i16, right: i16) {
        self.max_left = left;
        self.max_right = right;
    }

    pub fn set_num_system_words(&mut self, num: usize) {
        self.num_system = num;
    }

    pub fn preload_pos(&mut self, grammar: &Grammar) {
        assert_eq!(self.pos.len(), 0);
        for (i, pos) in grammar.pos_list.iter().enumerate() {
            let key = StrPosEntry::from_built_pos(pos);
            self.pos.insert(key, i as u16);
        }
        self.start_pos = self.pos.len();
    }

    pub(crate) fn pos_obj(&self, pos_id: u16) -> Option<&StrPosEntry> {
        self.pos.get_index(pos_id as usize).map(|(k, v)| {
            assert_eq!(v, &pos_id);
            k
        })
    }

    pub fn read_file(&mut self, path: &Path) -> SudachiResult<usize> {
        let file = File::open(path)?;
        let map = unsafe { Mmap::map(&file) }?;
        let filename = path.to_str().unwrap_or("<invalid-utf8>").to_owned();
        let old_name = self.ctx.set_filename(filename);
        let res = self.read_bytes(&map);
        self.ctx.set_filename(old_name);
        res
    }

    pub fn read_bytes(&mut self, data: &[u8]) -> SudachiResult<usize> {
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(false)
            .trim(Trim::None)
            .flexible(true)
            .from_reader(data);
        let mut record = StringRecord::new();
        let mut nread = 0;
        while reader.read_record(&mut record).map_err(|e| {
            let line = e.position().map_or(0, |p| p.line());
            self.ctx.set_line(line as usize);
            self.ctx.to_sudachi_err(BuildFailure::CsvError(e))
        })? {
            let line = record.position().map_or(0, |p| p.line()) as usize;
            self.ctx.set_line(line);
            self.read_record(&record)?;
            nread += 1;
        }
        Ok(nread)
    }

    fn read_record(&mut self, data: &StringRecord) -> SudachiResult<()> {
        self.parse_record(data).map(|r| self.entries.push(r))
    }

    fn parse_record(&mut self, data: &StringRecord) -> SudachiResult<RawLexiconEntry> {
        let ctx = std::mem::take(&mut self.ctx);
        let rec = RecordWrapper { record: data, ctx };
        let surface = rec.get(0, "(0) surface", unescape)?;
        let left_id = rec.get(1, "(1) left_id", parse_i16)?;
        let right_id = rec.get(2, "(2) right_id", parse_i16)?;
        let cost = rec.get(3, "(3) cost", parse_i16)?;

        let headword = rec.get(4, "(4) headword", unescape_cow)?;

        let p1 = rec.get(5, "(5) pos-1", unescape_cow)?;
        let p2 = rec.get(6, "(6) pos-2", unescape_cow)?;
        let p3 = rec.get(7, "(7) pos-3", unescape_cow)?;
        let p4 = rec.get(8, "(8) pos-4", unescape_cow)?;
        let p5 = rec.get(9, "(9) pos-conj-1", unescape_cow)?;
        let p6 = rec.get(10, "(10) pos-conj-2", unescape_cow)?;

        let reading = rec.get(11, "(11) reading", unescape_cow)?;
        let normalized = rec.get(12, "(12) normalized", unescape_cow)?;
        let dic_form_id = rec.get(13, "(13) dic-form", parse_dic_form)?;
        let splitting = rec.get(14, "(14) splitting", parse_mode)?;
        let (split_a, resolve_a) = rec.get(15, "(15) split-a", |s| self.parse_splits(s))?;
        let (split_b, resolve_b) = rec.get(16, "(16) split-b", |s| self.parse_splits(s))?;
        let parts = rec.get(17, "(17) word-structure", parse_wordid_list)?;
        let synonyms = rec.get_or_default(18, "(18) synonym-group", parse_u32_list)?;

        let pos = rec.ctx.transform(self.pos_of([p1, p2, p3, p4, p5, p6]))?;

        if splitting == Mode::A {
            if !split_a.is_empty() || !split_b.is_empty() {
                return rec.ctx.err(BuildFailure::InvalidSplit(
                    "A-mode tokens can't have splits".to_owned(),
                ));
            }
        }

        self.unresolved += resolve_a + resolve_b;

        if surface.is_empty() {
            return rec.ctx.err(BuildFailure::EmptySurface);
        }

        self.ctx = rec.ctx;

        let entry = RawLexiconEntry {
            left_id,
            right_id,
            cost,
            dic_form: dic_form_id,
            norm_form: none_if_equal(&headword, normalized),
            reading: none_if_equal(&headword, reading),
            headword: none_if_equal(&surface, headword),
            surface,
            pos,
            splitting,
            splits_a: split_a,
            splits_b: split_b,
            word_structure: parts,
            synonym_groups: synonyms,
        };

        Ok(entry)
    }

    fn pos_of(&mut self, data: [Cow<str>; POS_DEPTH]) -> DicWriteResult<u16> {
        match self.pos.get(&data) {
            Some(pos) => Ok(*pos),
            None => {
                let key = StrPosEntry::new(data);
                let pos_id = self.pos.len();
                if pos_id > MAX_POS_IDS {
                    Err(BuildFailure::PosLimitExceeded(format!("{:?}", key)))
                } else {
                    let pos_id = pos_id as u16;
                    self.pos.insert(key, pos_id);
                    Ok(pos_id)
                }
            }
        }
    }

    pub fn validate_entries(&self) -> SudachiResult<()> {
        let mut ctx = DicCompilationCtx::default();
        ctx.set_filename("<entry id>".to_owned());
        ctx.set_line(0);
        let (max_0, max_1) = match self.num_system {
            // means that we compile system dictionary, there must not be user words
            usize::MAX => (self.entries.len(), 0),
            // compiling user dictionary
            x => (x, self.entries.len()),
        };
        for e in self.entries.iter() {
            if e.left_id >= self.max_left {
                return ctx.err(BuildFailure::InvalidFieldSize {
                    actual: e.left_id as _,
                    expected: self.max_left as _,
                    field: "left_id",
                });
            }

            if e.right_id >= self.max_right {
                return ctx.err(BuildFailure::InvalidFieldSize {
                    actual: e.right_id as _,
                    expected: self.max_right as _,
                    field: "right_id",
                });
            }

            if e.dic_form != WordId::INVALID {
                ctx.transform(Self::validate_wid(e.dic_form, max_0, max_1, "dic_form"))?;
            }

            for s in e.splits_a.iter() {
                match s {
                    SplitUnit::Ref(wid) => {
                        ctx.transform(Self::validate_wid(*wid, max_0, max_1, "splits_a"))?;
                    }
                    _ => panic!("at this point there must not be unresolved splits"),
                }
            }

            for s in e.splits_b.iter() {
                match s {
                    SplitUnit::Ref(wid) => {
                        ctx.transform(Self::validate_wid(*wid, max_0, max_1, "splits_b"))?;
                    }
                    _ => panic!("at this point there must not be unresolved splits"),
                }
            }

            for wid in e.word_structure.iter() {
                ctx.transform(Self::validate_wid(*wid, max_0, max_1, "word_structure"))?;
            }

            ctx.add_line(1);
        }
        Ok(())
    }

    fn validate_wid(
        wid: WordId,
        dic0_max: usize,
        dic1_max: usize,
        label: &'static str,
    ) -> DicWriteResult<()> {
        let max = match wid.dic() {
            0 => dic0_max,
            1 => dic1_max,
            x => panic!("invalid dictionary ID={}, should not happen", x),
        };
        if wid.word() >= max as u32 {
            return Err(BuildFailure::InvalidFieldSize {
                actual: wid.word() as _,
                expected: max,
                field: label,
            });
        }
        Ok(())
    }

    fn parse_splits(&mut self, data: &str) -> DicWriteResult<(Vec<SplitUnit>, usize)> {
        if data.is_empty() || data == "*" {
            return Ok((Vec::new(), 0));
        }

        parse_slash_list(data, |s| self.parse_split(s)).map(|splits| {
            let unresolved = splits
                .iter()
                .map(|s| match s {
                    SplitUnit::Inline { .. } => 1,
                    _ => 0,
                })
                .sum();
            (splits, unresolved)
        })
    }

    fn parse_split(&mut self, data: &str) -> DicWriteResult<SplitUnit> {
        if WORD_ID_LITERAL.is_match(data) {
            Ok(SplitUnit::Ref(parse_wordid(data)?))
        } else {
            let mut iter = data.splitn(8, ",");
            let surface = it_next(data, &mut iter, "(1) surface", unescape)?;
            let p1 = it_next(data, &mut iter, "(2) pos-1", unescape_cow)?;
            let p2 = it_next(data, &mut iter, "(3) pos-2", unescape_cow)?;
            let p3 = it_next(data, &mut iter, "(4) pos-3", unescape_cow)?;
            let p4 = it_next(data, &mut iter, "(5) pos-4", unescape_cow)?;
            let p5 = it_next(data, &mut iter, "(6) pos-conj-1", unescape_cow)?;
            let p6 = it_next(data, &mut iter, "(7) pos-conj-2", unescape_cow)?;
            let reading = it_next(data, &mut iter, "(8) surface", unescape_cow)?;

            let pos = self.pos_of([p1, p2, p3, p4, p5, p6])?;
            Ok(SplitUnit::Inline {
                pos,
                reading: none_if_equal(&surface, reading),
                surface,
            })
        }
    }

    pub fn write_pos_table<W: Write>(&self, w: &mut W) -> SudachiResult<usize> {
        let mut u16w = Utf16Writer::new();
        let real_count = self.pos.len() - self.start_pos;
        w.write_all(&u16::to_le_bytes(real_count as u16))?;
        let mut written_bytes = 2;
        let mut ctx = DicCompilationCtx::default();
        ctx.set_filename("<pos-table>".to_owned());
        for (row, pos_id) in self.pos.iter() {
            if (*pos_id as usize) < self.start_pos {
                continue;
            }
            for field in row.fields() {
                ctx.apply(|| u16w.write(w, field).map(|written| written_bytes += written))?;
            }
            ctx.add_line(1);
        }
        Ok(written_bytes)
    }

    //noinspection DuplicatedCode
    pub(crate) fn resolve_splits<R: SplitUnitResolver>(
        &mut self,
        resolver: &R,
    ) -> Result<usize, (String, usize)> {
        let mut total = 0;
        let mut line: usize = 0;
        for e in self.entries.iter_mut() {
            for s in e.splits_a.iter_mut() {
                match Self::resolve_split(s, resolver) {
                    Some(val) => total += val,
                    None => {
                        // at this point s is a read only borrow,
                        // but borrow checker does not allow to do this cleanly
                        // self conflicts with splits_a borrow
                        let s: &SplitUnit = unsafe { std::mem::transmute(&*s) };
                        let split_info = s.format(self);
                        return Err((split_info, line));
                    }
                }
            }
            for s in e.splits_b.iter_mut() {
                match Self::resolve_split(s, resolver) {
                    Some(val) => total += val,
                    None => {
                        // at this point s is a read only borrow,
                        // but borrow checker does not allow to do this cleanly
                        // self conflicts with splits_b borrow
                        let s: &SplitUnit = unsafe { std::mem::transmute(&*s) };
                        let split_info = s.format(self);
                        return Err((split_info, line));
                    }
                }
            }
            line += 1;
        }
        Ok(total)
    }

    fn resolve_split<R: SplitUnitResolver>(unit: &mut SplitUnit, resolver: &R) -> Option<usize> {
        match unit {
            SplitUnit::Ref(_) => Some(0),
            _ => {
                let wid = resolver.resolve(&*unit)?;
                *unit = SplitUnit::Ref(wid);
                Some(1)
            }
        }
    }
}

struct RecordWrapper<'a> {
    pub record: &'a StringRecord,
    pub ctx: DicCompilationCtx,
}

impl<'a> RecordWrapper<'a> {
    #[inline(always)]
    fn get<T, F>(&self, idx: usize, name: &'static str, f: F) -> SudachiResult<T>
    where
        F: FnOnce(&'a str) -> DicWriteResult<T>,
    {
        match self.record.get(idx) {
            Some(s) => self.ctx.transform(f(s)),
            None => self.ctx.err(BuildFailure::NoRawField(name)),
        }
    }

    #[inline(always)]
    fn get_or_default<T, F>(&self, idx: usize, _name: &'static str, f: F) -> SudachiResult<T>
    where
        F: FnOnce(&'a str) -> DicWriteResult<T>,
        T: Default,
    {
        match self.record.get(idx) {
            Some(s) => self.ctx.transform(f(s)),
            None => Ok(<T as Default>::default()),
        }
    }
}

pub struct LexiconWriter<'a> {
    entries: &'a [RawLexiconEntry],
    u16: Utf16Writer,
    buffer: Vec<u8>,
    offset: usize,
    reporter: &'a mut Reporter,
}

impl<'a> LexiconWriter<'a> {
    pub(crate) fn new(
        entries: &'a [RawLexiconEntry],
        offset: usize,
        reporter: &'a mut Reporter,
    ) -> Self {
        Self {
            buffer: Vec::with_capacity(entries.len() * 32),
            entries,
            u16: Utf16Writer::new(),
            offset,
            reporter,
        }
    }

    pub fn write<W: Write>(&mut self, w: &mut W) -> SudachiResult<usize> {
        let mut ctx = DicCompilationCtx::memory();
        ctx.set_filename("<write entries>".to_owned());
        let mut total = 4;

        let num_entries = self.entries.len() as u32;
        w.write_all(&num_entries.to_le_bytes())?;

        let rep = ReportBuilder::new("word_params");
        ctx.set_line(0);
        for e in self.entries {
            total += ctx.transform(e.write_params(w))?;
            ctx.add_line(1);
        }
        self.reporter.collect(total, rep);
        let start = total;

        let rep = ReportBuilder::new("wordinfo_offsets");
        ctx.set_line(0);
        let offset_base = self.offset + (6 + 4) * self.entries.len() + 4;
        let mut word_offset = 0;
        for e in self.entries {
            let u32_offset = (offset_base + word_offset) as u32;
            w.write_all(&u32_offset.to_le_bytes())?;
            let size = ctx.transform(e.write_word_info(&mut self.u16, &mut self.buffer))?;
            word_offset += size;
            total += 4;
            ctx.add_line(1);
        }
        self.reporter.collect(total - start, rep);

        let rep = ReportBuilder::new("wordinfos (copy only)");
        let info_size = self.buffer.len();
        w.write_all(&self.buffer)?;
        self.reporter.collect(info_size, rep);

        Ok(total + info_size)
    }
}
