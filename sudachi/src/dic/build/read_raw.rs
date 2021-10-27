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

use crate::analysis::Mode;
use crate::dic::build::error::DicWriteReason::{InvalidCharLiteral, NoRawField};
use crate::dic::build::error::{DicCompilationCtx, DicWriteReason, DicWriteResult};
use crate::dic::build::{MAX_DIC_STRING_LEN, MAX_POS_IDS};
use crate::dic::word_id::WordId;
use crate::dic::POS_DEPTH;
use crate::error::SudachiResult;
use csv::{StringRecord, Trim};
use indexmap::map::IndexMap;
use indexmap::Equivalent;
use lazy_static::lazy_static;
use memmap2::Mmap;
use nom::error::ErrorKind::HexDigit;
use regex::{Match, Regex};
use std::borrow::{Borrow, Cow};
use std::collections::HashMap;
use std::fs::File;
use std::num::ParseIntError;
use std::path::Path;
use std::str::FromStr;

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
}

pub struct LexiconReader {
    pos: IndexMap<StrPosEntry, u16>,
    ctx: DicCompilationCtx,
    entries: Vec<RawLexiconEntry>,
}

pub(crate) enum SplitUnit {
    Ref(WordId),
    Inline {
        surface: String,
        pos: u16,
        reading: Option<String>,
    },
}

pub(crate) struct RawLexiconEntry {
    pub register: bool,
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
    pub splitting: Mode,
    pub word_structure: Vec<WordId>,
}

impl LexiconReader {
    pub fn read_file(&mut self, path: &Path) -> SudachiResult<()> {
        let file = File::open(path)?;
        let map = unsafe { Mmap::map(&file) }?;
        let old_name = self
            .ctx
            .set_filename(path.to_str().unwrap_or("failed").to_owned());
        let res = self.read_bytes(&map);
        self.ctx.set_filename(old_name);
        res
    }

    pub fn read_bytes(&mut self, data: &[u8]) -> SudachiResult<()> {
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(false)
            .trim(Trim::None)
            .flexible(true)
            .from_reader(data);
        let mut record = StringRecord::new();
        while reader.read_record(&mut record).map_err(|e| {
            let line = e.position().map_or(0, |p| p.line());
            self.ctx.set_line(line as usize);
            self.ctx.to_sudachi_err(DicWriteReason::CsvError(e))
        })? {
            let line = record.position().map_or(0, |p| p.line()) as usize;
            self.ctx.set_line(line);
            self.read_record(&record)?;
        }
        Ok(())
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
        let splitting = rec.get(14, "(14) splitting", mode)?;
        let split_a = rec.get(15, "(15) split-a", |s| self.parse_splits(s))?;
        let split_b = rec.get(16, "(16) split-b", |s| self.parse_splits(s))?;
        let parts = rec.get(17, "(17) word-structure", parse_wordid_list)?;

        let pos = rec.ctx.transform(self.pos_of([p1, p2, p3, p4, p5, p6]))?;

        if splitting == Mode::A {
            if !split_a.is_empty() || !split_b.is_empty() {
                return rec.ctx.err(DicWriteReason::InvalidSplit(
                    "A-mode tokens can't have splits".to_owned(),
                ));
            }
        }

        self.ctx = rec.ctx;

        Ok(RawLexiconEntry {
            register: left_id >= 0,
            left_id,
            right_id,
            cost,
            headword: none_if_equal(&surface, headword),
            dic_form: dic_form_id,
            norm_form: none_if_equal(&surface, normalized),
            reading: none_if_equal(&surface, reading),
            surface,
            pos,
            splitting,
            splits_a: split_a,
            splits_b: split_b,
            word_structure: parts,
        })
    }

    fn pos_of(&mut self, data: [Cow<str>; POS_DEPTH]) -> DicWriteResult<u16> {
        match self.pos.get(&data) {
            Some(pos) => Ok(*pos),
            None => {
                let key = StrPosEntry::new(data);
                let pos_id = self.pos.len();
                if pos_id > MAX_POS_IDS {
                    Err(todo!())
                } else {
                    let pos_id = pos_id as u16;
                    self.pos.insert(key, pos_id);
                    Ok(pos_id)
                }
            }
        }
    }

    fn parse_splits(&mut self, data: &str) -> DicWriteResult<Vec<SplitUnit>> {
        if data.is_empty() || data == "*" {
            return Ok(Vec::new());
        }

        parse_slash_list(data, |s| self.parse_split(s))
    }

    fn parse_split(&mut self, data: &str) -> DicWriteResult<SplitUnit> {
        if WORD_ID_LITERAL.is_match(data) {
            Ok(SplitUnit::Ref(parse_wordid(data)?))
        } else {
            let mut iter = data.splitn(8, ",");
            let surface = get_next(data, &mut iter, "(1) surface", unescape)?;
            let p1 = get_next(data, &mut iter, "(2) pos-1", unescape_cow)?;
            let p2 = get_next(data, &mut iter, "(3) pos-2", unescape_cow)?;
            let p3 = get_next(data, &mut iter, "(4) pos-3", unescape_cow)?;
            let p4 = get_next(data, &mut iter, "(5) pos-4", unescape_cow)?;
            let p5 = get_next(data, &mut iter, "(6) pos-conj-1", unescape_cow)?;
            let p6 = get_next(data, &mut iter, "(7) pos-conj-2", unescape_cow)?;
            let reading = get_next(data, &mut iter, "(8) surface", unescape_cow)?;

            let pos = self.pos_of([p1, p2, p3, p4, p5, p6])?;
            Ok(SplitUnit::Inline {
                pos,
                reading: none_if_equal(&surface, reading),
                surface,
            })
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
            None => self.ctx.err(NoRawField(name)),
        }
    }
}

#[inline(always)]
fn get_next<'a, I, T, F>(
    orig: &'a str,
    data: &mut I,
    field: &'static str,
    f: F,
) -> DicWriteResult<T>
where
    I: Iterator<Item = &'a str>,
    F: FnOnce(&'a str) -> DicWriteResult<T>,
    T: 'a,
{
    match data.next() {
        Some(s) => f(s),
        None => Err(DicWriteReason::SplitFormatError {
            original: orig.to_owned(),
            field,
        }),
    }
}

fn none_if_equal(surface: &str, data: Cow<str>) -> Option<String> {
    if surface == data {
        None
    } else {
        match data {
            Cow::Borrowed(x) => Some(x.to_owned()),
            Cow::Owned(x) => Some(x),
        }
    }
}

#[inline]
fn mode(data: &str) -> DicWriteResult<Mode> {
    match data.trim() {
        "a" | "A" => Ok(Mode::A),
        "b" | "B" => Ok(Mode::B),
        "c" | "C" | "*" => Ok(Mode::C),
        _ => Err(DicWriteReason::InvalidSplit(data.to_owned())),
    }
}

#[inline]
fn parse_i16(data: &str) -> DicWriteResult<i16> {
    match i16::from_str(data) {
        Ok(v) => Ok(v),
        Err(_) => Err(DicWriteReason::InvalidI16Literal(data.to_owned())),
    }
}

#[inline]
fn parse_dic_form(data: &str) -> DicWriteResult<WordId> {
    if data == "*" {
        Ok(WordId::INVALID)
    } else {
        parse_wordid(data)
    }
}

#[inline]
fn parse_wordid(data: &str) -> DicWriteResult<WordId> {
    if data.starts_with("U") {
        let wid = parse_wordid_raw(&data[1..]);
        wid.map(|w| WordId::new(1, w.word()))
    } else {
        parse_wordid_raw(data)
    }
}

#[inline]
fn parse_wordid_raw(data: &str) -> DicWriteResult<WordId> {
    match u32::from_str(data) {
        Ok(v) => match WordId::checked(0, v) {
            Ok(id) => Ok(id),
            Err(_) => Err(DicWriteReason::InvalidWordId(data.to_owned())),
        },
        Err(_) => Err(DicWriteReason::InvalidWordId(data.to_owned())),
    }
}

#[inline]
fn parse_wordid_list(data: &str) -> DicWriteResult<Vec<WordId>> {
    if data.len() == 0 || data == "*" {
        return Ok(Vec::new());
    }

    parse_slash_list(data, parse_wordid)
}

lazy_static! {
    static ref WORD_ID_LITERAL: Regex = Regex::new(r"U?\d+").unwrap();
}

#[inline]
fn parse_slash_list<T, F>(data: &str, mut f: F) -> DicWriteResult<Vec<T>>
where
    F: FnMut(&str) -> DicWriteResult<T>,
{
    let mut data = data;
    let mut start = 0;
    let mut result = Vec::with_capacity(4);
    while start < data.len() {
        let end = data.find("/").unwrap_or(data.len());
        result.push(f(&data[..end])?);
        start = end;
        data = &data[(end + 1)..]; // skip slash
    }
    Ok(result)
}

lazy_static! {
    static ref UNICODE_LITERAL: Regex =
        Regex::new(r"\\u(?:\{([0-9a-fA-F]{1,6})\}|([0-9a-fA-F]{4}))").unwrap();
}

fn check_len(data: &str) -> DicWriteResult<()> {
    if data.len() > MAX_DIC_STRING_LEN {
        Err(DicWriteReason::InvalidSize {
            expected: MAX_DIC_STRING_LEN,
            actual: data.len(),
        })
    } else {
        Ok(())
    }
}

#[inline]
fn unescape_cow(data: &str) -> DicWriteResult<Cow<str>> {
    check_len(data)?;
    if !UNICODE_LITERAL.is_match(data) {
        Ok(Cow::Borrowed(data))
    } else {
        unescape_slow(data).map(|s| Cow::Owned(s))
    }
}

#[inline]
fn unescape(data: &str) -> DicWriteResult<String> {
    check_len(data)?;
    if !UNICODE_LITERAL.is_match(data) {
        Ok(data.to_owned())
    } else {
        unescape_slow(data)
    }
}

#[inline(never)]
fn unescape_slow(original: &str) -> DicWriteResult<String> {
    let mut result = String::with_capacity(original.len());
    let mut start = 0;
    for c in UNICODE_LITERAL.captures_iter(original) {
        let whole = c.get(0).unwrap();
        let braces = c.get(1).or_else(|| c.get(2)).unwrap();
        result.push_str(&original[start..whole.start()]);
        match u32::from_str_radix(braces.as_str(), 16) {
            Ok(c) => match char::from_u32(c) {
                Some(cx) => result.push(cx),
                None => return Err(InvalidCharLiteral(braces.as_str().to_owned())),
            },
            Err(_) => return Err(InvalidCharLiteral(braces.as_str().to_owned())),
        }
        start = whole.end();
    }
    result.push_str(&original[start..]);
    Ok(result)
}

mod test {
    use super::*;

    #[test]
    fn decode_plain() {
        assert_eq!(unescape("").unwrap(), "");
        assert_eq!(unescape("a").unwrap(), "a");
        assert_eq!(unescape("あ").unwrap(), "あ");
    }

    #[test]
    fn decode_escape_1() {
        assert_eq!(unescape("\\u0020").unwrap(), "\u{20}");
        assert_eq!(unescape("\\u{20}").unwrap(), "\u{20}");
        assert_eq!(unescape("\\u{1f49e}").unwrap(), "💞");
    }

    #[test]
    fn decode_escape_2() {
        assert_eq!(unescape("\\u020f").unwrap(), "\u{20f}");
        assert_eq!(unescape("\\u{20}f").unwrap(), "\u{20}f");
    }

    #[test]
    fn decode_escape_3() {
        assert_eq!(unescape("f\\u0020").unwrap(), "f\u{20}");
        assert_eq!(unescape("f\\u{20}").unwrap(), "f\u{20}");
    }

    #[test]
    fn decode_escape_4() {
        assert_eq!(unescape("\\u100056").unwrap(), "\u{1000}56");
    }

    #[test]
    fn decode_escape_ported() {
        assert_eq!(unescape("a\\u002cc").unwrap(), "a,c");
        assert_eq!(unescape("a\\u{2c}c").unwrap(), "a,c");
    }

    #[test]
    fn decode_escape_fail() {
        assert_eq!(unescape("\\u{10FFFF}").unwrap(), "\u{10FFFF}"); // max character
        claim::assert_matches!(unescape("\\u{110000}"), Err(_));
        claim::assert_matches!(unescape("\\u{FFFFFF}"), Err(_));
    }
}
