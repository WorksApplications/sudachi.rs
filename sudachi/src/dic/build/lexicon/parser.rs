/*
 *  Copyright (c) 2026 Works Applications Co., Ltd.
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

use std::borrow::Cow;
use std::fs::File;
use std::path::Path;

use csv::{StringRecord, Trim};
use memmap2::Mmap;

use crate::analysis::Mode;
use crate::dic::build::error::{BuildFailure, DicWriteResult};
use crate::dic::build::parse::{
    it_next, none_if_equal, parse_i16, parse_legacy_line_ref, parse_mode, parse_slash_list,
    parse_u32_list_with_asterisk, unescape, unescape_cow, WORD_ID_LITERAL,
};
use crate::dic::build::pos::read_pos_bytes as read_pos_csv_bytes;
use crate::dic::build::MAX_POS_IDS;
use crate::dic::pos::POS_DEPTH;
use crate::error::SudachiResult;

use super::layout::{Column, ColumnLayout, RecordWrapper};
use super::{LexiconReader, ParsedLexiconEntry, StrPosEntry, WordRef};

impl LexiconReader {
    pub fn read_pos_file(&mut self, path: &Path) -> SudachiResult<usize> {
        let file = File::open(path)?;
        let map = unsafe { Mmap::map(&file) }?;
        let filename = path.to_str().unwrap_or("<invalid-utf8>").to_owned();
        let old_name = self.ctx.set_filename(filename);
        let res = self.read_pos_bytes(&map);
        self.ctx.set_filename(old_name);
        res
    }

    pub fn read_pos_bytes(&mut self, data: &[u8]) -> SudachiResult<usize> {
        read_pos_csv_bytes(
            &mut self.pos,
            !self.parsed_entries.is_empty(),
            data,
            &mut self.ctx,
        )
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
        // This only parses and stores lexicon csv entries. Cross-entry references
        // must be resolved later via DictBuilder::resolve() before compilation.
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(false)
            .trim(Trim::None)
            .flexible(true)
            .from_reader(data);
        let mut layout = ColumnLayout::Legacy;
        let mut first_row = true;
        let mut nread = 0;
        for record in reader.records() {
            match record {
                Ok(r) => {
                    let line = r.position().map_or(0, |p| p.line()) as usize;
                    self.ctx.set_line(line);
                    if first_row {
                        first_row = false;
                        let (resolved, skip_row) = ColumnLayout::from_record(&r, &self.ctx)?;
                        layout = resolved;
                        if skip_row {
                            continue;
                        }
                    }
                    self.read_record(&r, layout)?;
                    nread += 1;
                }
                Err(e) => {
                    let line = e.position().map_or(0, |p| p.line()) as usize;
                    self.ctx.set_line(line);
                    return Err(self.ctx.to_sudachi_err(BuildFailure::CsvError(e)));
                }
            }
        }
        Ok(nread)
    }

    fn read_record(&mut self, data: &StringRecord, layout: ColumnLayout) -> SudachiResult<()> {
        self.parse_record(data, layout)
            .map(|r| self.parsed_entries.push(r))
    }

    fn parse_record(
        &mut self,
        data: &StringRecord,
        layout: ColumnLayout,
    ) -> SudachiResult<ParsedLexiconEntry> {
        let ctx = std::mem::take(&mut self.ctx);
        let rec = RecordWrapper { record: data, ctx };
        let index_form = rec.get_col(layout, Column::IndexForm, unescape)?;
        let left_id = rec.get_col(layout, Column::LeftId, parse_i16)?;
        let right_id = rec.get_col(layout, Column::RightId, parse_i16)?;
        let cost = rec.get_col(layout, Column::Cost, parse_i16)?;

        let headword = rec.get_col_or_empty(layout, Column::Headword, unescape_cow)?;

        let p1 = rec.get_col_or_empty(layout, Column::Pos1, unescape_cow)?;
        let p2 = rec.get_col_or_empty(layout, Column::Pos2, unescape_cow)?;
        let p3 = rec.get_col_or_empty(layout, Column::Pos3, unescape_cow)?;
        let p4 = rec.get_col_or_empty(layout, Column::Pos4, unescape_cow)?;
        let p5 = rec.get_col_or_empty(layout, Column::Pos5, unescape_cow)?;
        let p6 = rec.get_col_or_empty(layout, Column::Pos6, unescape_cow)?;

        let normalized = rec.get_col(layout, Column::NormalizedForm, |s| Ok(s.to_owned()))?;
        let dic_form_ref = rec.get_col_or(layout, Column::DictionaryForm, "".to_owned(), |s| {
            Ok(s.to_owned())
        })?;
        let splitting = rec.get_col_or(layout, Column::Mode, Mode::C, parse_mode)?;
        let allow_word_id_ref = layout.is_legacy();
        let allow_asterisk = layout.is_legacy();
        let splits_a = rec.get_col(layout, Column::SplitA, |s| {
            self.parse_splits_with_asterisk(s, allow_word_id_ref, allow_asterisk)
        })?;
        let splits_b = rec.get_col(layout, Column::SplitB, |s| {
            self.parse_splits_with_asterisk(s, allow_word_id_ref, allow_asterisk)
        })?;
        let splits_c = rec.get_col_or_default(layout, Column::SplitC, |s| {
            self.parse_splits_with_asterisk(s, allow_word_id_ref, allow_asterisk)
        })?;
        let word_structure = rec.get_col(layout, Column::WordStructure, |s| {
            self.parse_splits_with_asterisk(s, allow_word_id_ref, allow_asterisk)
        })?;
        let synonym_groups = rec.get_col_or_default(layout, Column::SynonymGroups, |s| {
            parse_u32_list_with_asterisk(s, allow_asterisk)
        })?;
        let user_data = rec.get_col_or_default(layout, Column::UserData, unescape)?;
        let pos_id = rec.get_col_or(layout, Column::PosId, -1_i16, |s| {
            if s.is_empty() {
                Ok(-1)
            } else {
                parse_i16(s)
            }
        })?;
        let reference_id = rec.get_col_or(layout, Column::ReferenceId, String::new(), |s| {
            Ok(s.to_owned())
        })?;

        let pos = if !p1.is_empty() {
            let pos = rec
                .ctx
                .transform(self.pos_id_of([p1, p2, p3, p4, p5, p6]))?;
            if pos_id >= 0 && pos_id as u16 != pos {
                return rec.ctx.err(BuildFailure::InvalidSplit(
                    "PosId and Pos1..Pos6 do not match".to_owned(),
                ));
            }
            pos
        } else if pos_id >= 0 {
            let pos = pos_id as u16;
            if pos as usize >= self.pos.len() {
                return rec.ctx.err(BuildFailure::InvalidSplit(
                    "POS for id was not present in the table".to_owned(),
                ));
            }
            pos
        } else {
            return rec.ctx.err(BuildFailure::InvalidSplit(
                "Both PosId and Pos1..Pos6 are missing".to_owned(),
            ));
        };

        if splitting == Mode::A && (!splits_a.is_empty() || !splits_b.is_empty()) {
            return rec.ctx.err(BuildFailure::InvalidSplit(
                "A-mode tokens can't have splits".to_owned(),
            ));
        }

        if left_id >= self.max_left {
            return rec.ctx.err(BuildFailure::InvalidFieldSize {
                actual: left_id as _,
                expected: self.max_left as _,
                field: "left_id",
            });
        }

        if right_id >= self.max_right {
            return rec.ctx.err(BuildFailure::InvalidFieldSize {
                actual: right_id as _,
                expected: self.max_right as _,
                field: "right_id",
            });
        }

        let effective_headword: Cow<str> = if headword.is_empty() {
            Cow::Borrowed(index_form.as_str())
        } else {
            headword
        };

        let reading = rec.get_col(layout, Column::ReadingForm, unescape_cow)?;
        let reading_field = none_if_equal(effective_headword.as_ref(), reading);
        let dic_form = rec
            .ctx
            .transform(self.parse_dic_form(&dic_form_ref, allow_word_id_ref))?;
        let norm_form = rec.ctx.transform(self.parse_norm_form(
            &normalized,
            effective_headword.as_ref(),
            pos,
            reading_field.as_deref(),
            if reference_id.is_empty() {
                None
            } else {
                Some(reference_id.as_str())
            },
            layout.is_legacy(),
        ))?;

        if index_form.is_empty() {
            return rec.ctx.err(BuildFailure::EmptyIndexForm);
        }

        self.ctx = rec.ctx;

        Ok(ParsedLexiconEntry {
            left_id,
            right_id,
            cost,
            dic_form,
            norm_form,
            reading: reading_field,
            headword: none_if_equal(&index_form, effective_headword),
            reference_id: if reference_id.is_empty() {
                None
            } else {
                Some(reference_id)
            },
            index_form,
            pos,
            splitting,
            splits_a,
            splits_b,
            splits_c,
            word_structure,
            synonym_groups,
            user_data,
        })
    }

    fn pos_id_of(&mut self, data: [Cow<str>; POS_DEPTH]) -> DicWriteResult<u16> {
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

    #[cfg(test)]
    pub(super) fn parse_splits(
        &mut self,
        data: &str,
        allow_word_id_ref: bool,
    ) -> DicWriteResult<Vec<WordRef>> {
        self.parse_splits_with_asterisk(data, allow_word_id_ref, true)
    }

    fn parse_splits_with_asterisk(
        &mut self,
        data: &str,
        allow_word_id_ref: bool,
        allow_asterisk: bool,
    ) -> DicWriteResult<Vec<WordRef>> {
        if data.is_empty() || data == "*" {
            if data == "*" && !allow_asterisk {
                return Err(BuildFailure::InvalidSplit(data.to_owned()));
            }
            return Ok(Vec::new());
        }

        parse_slash_list(data, |s| self.parse_split(s, allow_word_id_ref))
    }

    fn parse_split(&mut self, data: &str, allow_word_id_ref: bool) -> DicWriteResult<WordRef> {
        if WORD_ID_LITERAL.is_match(data) {
            if !allow_word_id_ref {
                return Err(BuildFailure::InvalidSplit(data.to_owned()));
            }
            Ok(WordRef::LineRef(parse_legacy_line_ref(data)?))
        } else if matches!(data.matches(',').count(), 2 | 3) {
            let mut iter = data.splitn(4, ',');
            let headword = it_next(data, &mut iter, "(1) headword", unescape)?;
            let pos = it_next(data, &mut iter, "(2) pos-id", parse_i16)?;
            let reading = it_next(data, &mut iter, "(3) reading", unescape_cow)?;
            let reference_id = match iter.next() {
                None => None,
                Some("") => return Err(BuildFailure::InvalidSplit(data.to_owned())),
                Some(value) => Some(unescape(value)?),
            };
            self.make_entry_key(data, headword, pos as u16, reading, reference_id)
        } else {
            let mut iter = data.splitn(9, ',');
            let headword = it_next(data, &mut iter, "(1) headword", unescape)?;
            let p1 = it_next(data, &mut iter, "(2) pos-1", unescape_cow)?;
            let p2 = it_next(data, &mut iter, "(3) pos-2", unescape_cow)?;
            let p3 = it_next(data, &mut iter, "(4) pos-3", unescape_cow)?;
            let p4 = it_next(data, &mut iter, "(5) pos-4", unescape_cow)?;
            let p5 = it_next(data, &mut iter, "(6) pos-conj-1", unescape_cow)?;
            let p6 = it_next(data, &mut iter, "(7) pos-conj-2", unescape_cow)?;
            let reading = it_next(data, &mut iter, "(8) reading", unescape_cow)?;
            let reference_id = match iter.next() {
                None => None,
                Some("") => return Err(BuildFailure::InvalidSplit(data.to_owned())),
                Some(value) => Some(unescape(value)?),
            };

            let pos = self.pos_id_of([p1, p2, p3, p4, p5, p6])?;
            self.make_entry_key(data, headword, pos, reading, reference_id)
        }
    }

    fn parse_dic_form(&mut self, data: &str, allow_word_id_ref: bool) -> DicWriteResult<WordRef> {
        if data.is_empty() || (allow_word_id_ref && data == "*") {
            return Ok(WordRef::SelfRef);
        }
        if data == "*" {
            return Err(BuildFailure::InvalidSplit(data.to_owned()));
        }

        self.parse_split(data, allow_word_id_ref)
    }

    fn parse_norm_form(
        &mut self,
        data: &str,
        headword: &str,
        pos: u16,
        reading: Option<&str>,
        reference_id: Option<&str>,
        allow_asterisk: bool,
    ) -> DicWriteResult<WordRef> {
        if data.is_empty() || (allow_asterisk && data == "*") {
            return Ok(WordRef::SelfRef);
        }

        if matches!(data.matches(',').count(), 2 | 3 | 7 | 8) {
            let word_ref = self.parse_split(data, false)?;
            if word_ref.matches(headword, pos, reading, reference_id) {
                return Ok(WordRef::SelfRef);
            }
            return Ok(word_ref);
        }

        let normalized = unescape(data)?;
        if normalized == headword {
            Ok(WordRef::SelfRef)
        } else {
            Ok(WordRef::Headword(normalized))
        }
    }

    fn make_entry_key(
        &self,
        original: &str,
        headword: String,
        pos: u16,
        reading: Cow<'_, str>,
        reference_id: Option<String>,
    ) -> DicWriteResult<WordRef> {
        if headword.is_empty() || reading.is_empty() {
            return Err(BuildFailure::InvalidSplit(original.to_owned()));
        }

        Ok(WordRef::EntryKey {
            pos,
            reading: none_if_equal(&headword, reading),
            headword,
            reference_id,
        })
    }
}
