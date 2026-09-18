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

use std::io::Write;

use crate::dic::build::error::{BuildFailure, DicCompilationCtx, DicWriteResult};
use crate::dic::build::report::{ReportBuilder, Reporter};
use crate::dic::lexicon::strings::StringPointer;
use crate::dic::word_id::WordRef;
use crate::dic::word_info::{WordInfoFixedData, WordInfoVariableData, WordInfos};
use crate::error::SudachiResult;

use super::entry::ResolvedLexiconEntry;
use super::{LexiconReader, ResolvedWordRef, StringStore};

pub struct LexiconWriter<'a> {
    entries: &'a [ResolvedLexiconEntry],
    strings: &'a StringStore,
    user: bool,
    reporter: &'a mut Reporter,
}

impl<'a> LexiconWriter<'a> {
    pub(crate) fn new(
        entries: &'a [ResolvedLexiconEntry],
        strings: &'a StringStore,
        user: bool,
        reporter: &'a mut Reporter,
    ) -> Self {
        Self {
            entries,
            strings,
            user,
            reporter,
        }
    }

    pub fn write<W: Write>(&mut self, w: &mut W) -> SudachiResult<usize> {
        let mut ctx = DicCompilationCtx::memory();
        ctx.set_filename("<write entries>".to_owned());
        let mut total = LexiconReader::ENTRY_INITIAL_OFFSET;
        w.write_all(&[0u8; LexiconReader::ENTRY_INITIAL_OFFSET])?;

        let rep = ReportBuilder::new("entries");
        ctx.set_line(0);
        let mut offset = LexiconReader::ENTRY_INITIAL_OFFSET;
        for e in self.entries {
            let entry_id = (offset >> WordInfos::WORD_ID_ALIGNMENT_BITS) as u32;
            let self_word_ref = WordRef::new(!self.user, entry_id);
            let headword_strptr = self.strings.resolve(e.headword());
            let reading_strptr = self.strings.resolve(e.reading());
            total += ctx.transform(e.write_params(w))?;
            total +=
                ctx.transform(e.write_rest(w, self_word_ref, headword_strptr, reading_strptr))?;
            let expected_end = offset + e.expected_entry_size();
            if total > expected_end {
                return ctx.err(BuildFailure::InvalidSize {
                    actual: total,
                    expected: expected_end,
                });
            }
            while total < expected_end {
                w.write_all(&[0])?;
                total += 1;
            }
            offset = expected_end;
            ctx.add_line(1);
        }
        self.reporter.collect(total, rep);
        Ok(total)
    }
}

impl LexiconReader {
    pub fn write_pos_table<W: Write>(&self, w: &mut W) -> SudachiResult<usize> {
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
                ctx.apply(|| write_short_utf16(w, field).map(|written| written_bytes += written))?;
            }
            ctx.add_line(1);
        }
        Ok(written_bytes)
    }
}

fn write_short_utf16<W: Write>(w: &mut W, data: &str) -> DicWriteResult<usize> {
    let utf16: Vec<u16> = data.encode_utf16().collect();
    if utf16.len() > i16::MAX as usize {
        return Err(BuildFailure::InvalidSize {
            actual: utf16.len(),
            expected: i16::MAX as usize,
        });
    }
    let len = utf16.len() as i16;
    w.write_all(&len.to_le_bytes())?;
    let mut written = 2;
    for c in utf16 {
        w.write_all(&c.to_le_bytes())?;
        written += 2;
    }
    Ok(written)
}

impl ResolvedLexiconEntry {
    pub fn write_params<W: Write>(&self, w: &mut W) -> DicWriteResult<usize> {
        w.write_all(&self.left_id.to_le_bytes())?;
        w.write_all(&self.right_id.to_le_bytes())?;
        w.write_all(&self.cost.to_le_bytes())?;
        Ok(6)
    }

    pub fn write_rest<W: Write>(
        &self,
        w: &mut W,
        self_word_ref: WordRef,
        headword_strptr: StringPointer,
        reading_strptr: StringPointer,
    ) -> DicWriteResult<usize> {
        let dic_form_word_ref = match self.dic_form {
            ResolvedWordRef::Ref(wref) => wref,
            ResolvedWordRef::SelfRef => self_word_ref,
        };
        let norm_form_word_ref = match self.norm_form {
            ResolvedWordRef::Ref(wref) => wref,
            ResolvedWordRef::SelfRef => self_word_ref,
        };

        if self.index_form.len() > i16::MAX as usize {
            return Err(BuildFailure::InvalidFieldSize {
                actual: self.index_form.len(),
                expected: i16::MAX as usize,
                field: "index_form_length",
            });
        }

        let variable = self.variable_layout()?;

        let fixed = WordInfoFixedData {
            pos_id: self.pos as i16,
            headword_strptr,
            reading_form_strptr: reading_strptr,
            normalized_form: norm_form_word_ref.as_raw(),
            dictionary_form: dic_form_word_ref.as_raw(),
            index_form_length: self.index_form.len() as i16,
            c_unit_split_length: variable.c_unit_split_length,
            b_unit_split_length: variable.b_unit_split_length,
            a_unit_split_length: variable.a_unit_split_length,
            word_structure_length: variable.word_structure_length,
            synonym_group_ids_length: variable.synonym_group_ids_length,
            user_data_flag: variable.user_data_flag,
        };
        let mut size = fixed.write_to(w)?;
        let c_refs = self.word_refs_to_raw(&self.splits_c);
        let b_refs = self.word_refs_to_raw(&self.splits_b);
        let a_refs = self.word_refs_to_raw(&self.splits_a);
        let ws_refs = self.word_refs_to_raw(&self.word_structure);
        let syns: Vec<i32> = self.synonym_groups.iter().map(|sg| *sg as i32).collect();
        let payload = WordInfoVariableData {
            c_unit_split: &c_refs,
            b_unit_split: &b_refs,
            a_unit_split: &a_refs,
            word_structure: &ws_refs,
            synonym_group_ids: &syns,
            user_data: &self.user_data,
        };
        size += payload.write_to(w, &fixed)?;

        Ok(size)
    }

    fn word_refs_to_raw(&self, refs: &[WordRef]) -> Vec<u32> {
        refs.iter().map(|wref| wref.as_raw()).collect()
    }
}
