/*
 *  Copyright (c) 2021-2026 Works Applications Co., Ltd.
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

use csv::StringRecord;
use lazy_static::lazy_static;
use regex::Regex;

use crate::dic::build::csv_schema::{parse_header_mapping, validate_required_columns, CsvColumn};
use crate::dic::build::error::{BuildFailure, DicCompilationCtx, DicWriteResult};
use crate::dic::pos::POS_DEPTH;
use crate::error::SudachiResult;

const NUM_COLUMNS: usize = 23;
lazy_static! {
    static ref INTEGER_LITERAL: Regex = Regex::new(r"^-?\d+$").unwrap();
}

#[repr(usize)]
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub(super) enum Column {
    IndexForm = 0,
    LeftId = 1,
    RightId = 2,
    Cost = 3,
    Headword = 4,
    Pos1 = 5,
    Pos2 = 6,
    Pos3 = 7,
    Pos4 = 8,
    Pos5 = 9,
    Pos6 = 10,
    ReadingForm = 11,
    NormalizedForm = 12,
    DictionaryForm = 13,
    Mode = 14,
    SplitA = 15,
    SplitB = 16,
    WordStructure = 17,
    SynonymGroups = 18,
    SplitC = 19,
    UserData = 20,
    PosId = 21,
    ReferenceId = 22,
}

impl Column {
    const fn v0_index(self) -> usize {
        self as usize
    }

    pub(super) const fn label(self) -> &'static str {
        match self {
            Column::IndexForm => "INDEX_FORM",
            Column::LeftId => "LEFT_ID",
            Column::RightId => "RIGHT_ID",
            Column::Cost => "COST",
            Column::Headword => "HEADWORD",
            Column::Pos1 => "POS1",
            Column::Pos2 => "POS2",
            Column::Pos3 => "POS3",
            Column::Pos4 => "POS4",
            Column::Pos5 => "POS5",
            Column::Pos6 => "POS6",
            Column::ReadingForm => "READING_FORM",
            Column::NormalizedForm => "NORMALIZED_FORM",
            Column::DictionaryForm => "DICTIONARY_FORM",
            Column::Mode => "MODE",
            Column::SplitA => "SPLIT_A",
            Column::SplitB => "SPLIT_B",
            Column::WordStructure => "WORD_STRUCTURE",
            Column::SynonymGroups => "SYNONYM_GROUPS",
            Column::SplitC => "SPLIT_C",
            Column::UserData => "USER_DATA",
            Column::PosId => "POS_ID",
            Column::ReferenceId => "REFERENCE_ID",
        }
    }
}

impl CsvColumn<NUM_COLUMNS> for Column {
    fn as_usize(self) -> usize {
        self as usize
    }

    fn label(self) -> &'static str {
        match self {
            Column::IndexForm => "INDEX_FORM",
            Column::LeftId => "LEFT_ID",
            Column::RightId => "RIGHT_ID",
            Column::Cost => "COST",
            Column::Headword => "HEADWORD",
            Column::Pos1 => "POS1",
            Column::Pos2 => "POS2",
            Column::Pos3 => "POS3",
            Column::Pos4 => "POS4",
            Column::Pos5 => "POS5",
            Column::Pos6 => "POS6",
            Column::ReadingForm => "READING_FORM",
            Column::NormalizedForm => "NORMALIZED_FORM",
            Column::DictionaryForm => "DICTIONARY_FORM",
            Column::Mode => "MODE",
            Column::SplitA => "SPLIT_A",
            Column::SplitB => "SPLIT_B",
            Column::WordStructure => "WORD_STRUCTURE",
            Column::SynonymGroups => "SYNONYM_GROUPS",
            Column::SplitC => "SPLIT_C",
            Column::UserData => "USER_DATA",
            Column::PosId => "POS_ID",
            Column::ReferenceId => "REFERENCE_ID",
        }
    }

    fn from_normalized(data: &str) -> Option<Self> {
        match data {
            "indexform" => Some(Column::IndexForm),
            "leftid" => Some(Column::LeftId),
            "rightid" => Some(Column::RightId),
            "cost" => Some(Column::Cost),
            "headword" => Some(Column::Headword),
            "pos1" => Some(Column::Pos1),
            "pos2" => Some(Column::Pos2),
            "pos3" => Some(Column::Pos3),
            "pos4" => Some(Column::Pos4),
            "pos5" => Some(Column::Pos5),
            "pos6" => Some(Column::Pos6),
            "readingform" => Some(Column::ReadingForm),
            "normalizedform" => Some(Column::NormalizedForm),
            "dictionaryform" => Some(Column::DictionaryForm),
            "mode" => Some(Column::Mode),
            "splita" => Some(Column::SplitA),
            "splitb" => Some(Column::SplitB),
            "wordstructure" => Some(Column::WordStructure),
            "synonymgroups" => Some(Column::SynonymGroups),
            "splitc" => Some(Column::SplitC),
            "userdata" => Some(Column::UserData),
            "posid" => Some(Column::PosId),
            "referenceid" => Some(Column::ReferenceId),
            _ => None,
        }
    }
}

const POS_PARTS: [Column; POS_DEPTH] = [
    Column::Pos1,
    Column::Pos2,
    Column::Pos3,
    Column::Pos4,
    Column::Pos5,
    Column::Pos6,
];

const REQUIRED_COLUMNS: [Column; 10] = [
    Column::IndexForm,
    Column::LeftId,
    Column::RightId,
    Column::Cost,
    Column::ReadingForm,
    Column::NormalizedForm,
    Column::DictionaryForm,
    Column::SplitA,
    Column::SplitB,
    Column::WordStructure,
];

#[derive(Copy, Clone)]
pub(super) enum ColumnLayout {
    V0,
    Header([i16; NUM_COLUMNS]),
}

impl ColumnLayout {
    pub(super) fn from_record(
        record: &StringRecord,
        ctx: &DicCompilationCtx,
    ) -> SudachiResult<(Self, bool)> {
        if record.len() > 1 {
            if let Some(left_id) = record.get(Column::LeftId.v0_index()) {
                if INTEGER_LITERAL.is_match(left_id) {
                    return Ok((ColumnLayout::V0, false));
                }
            }
        }

        let mapping = parse_header_mapping::<Column, NUM_COLUMNS>(
            record,
            ctx,
            "INVALID_COLUMN_NAME",
            "DUPLICATED_COLUMN_NAME",
        )?;
        validate_required_columns(&mapping, &REQUIRED_COLUMNS, ctx)?;

        let pos_parts_found = POS_PARTS
            .iter()
            .filter(|col| mapping[col.as_usize()] >= 0)
            .count();
        if pos_parts_found != 0 && pos_parts_found != POS_DEPTH {
            return ctx.err(BuildFailure::NoRawField("POS1_6_SET"));
        }

        let has_pos_id = mapping[Column::PosId.as_usize()] >= 0;
        if !has_pos_id && pos_parts_found == 0 {
            return ctx.err(BuildFailure::NoRawField("POS_OR_POS_ID"));
        }

        Ok((ColumnLayout::Header(mapping), true))
    }

    fn index(self, col: Column) -> Option<usize> {
        match self {
            ColumnLayout::V0 => Some(col.v0_index()),
            ColumnLayout::Header(mapping) => {
                let idx = mapping[col.as_usize()];
                if idx < 0 {
                    None
                } else {
                    Some(idx as usize)
                }
            }
        }
    }

    pub(super) const fn is_v0(self) -> bool {
        matches!(self, ColumnLayout::V0)
    }
}

pub(super) struct RecordWrapper<'a> {
    pub record: &'a StringRecord,
    pub ctx: DicCompilationCtx,
}

impl<'a> RecordWrapper<'a> {
    #[inline(always)]
    pub(super) fn get_col<T, F>(&self, layout: ColumnLayout, col: Column, f: F) -> SudachiResult<T>
    where
        F: FnOnce(&'a str) -> DicWriteResult<T>,
    {
        match layout.index(col).and_then(|idx| self.record.get(idx)) {
            Some(s) => self.ctx.transform(f(s)),
            None => self.ctx.err(BuildFailure::NoRawField(col.label())),
        }
    }

    #[inline(always)]
    pub(super) fn get_col_or_empty<T, F>(
        &self,
        layout: ColumnLayout,
        col: Column,
        f: F,
    ) -> SudachiResult<T>
    where
        F: FnOnce(&'a str) -> DicWriteResult<T>,
    {
        match layout.index(col).and_then(|idx| self.record.get(idx)) {
            Some(s) => self.ctx.transform(f(s)),
            None => self.ctx.transform(f("")),
        }
    }

    #[inline(always)]
    pub(super) fn get_col_or_default<T, F>(
        &self,
        layout: ColumnLayout,
        col: Column,
        f: F,
    ) -> SudachiResult<T>
    where
        F: FnOnce(&'a str) -> DicWriteResult<T>,
        T: Default,
    {
        self.get_col_or(layout, col, T::default(), f)
    }

    #[inline(always)]
    pub(super) fn get_col_or<T, F>(
        &self,
        layout: ColumnLayout,
        col: Column,
        default: T,
        f: F,
    ) -> SudachiResult<T>
    where
        F: FnOnce(&'a str) -> DicWriteResult<T>,
    {
        match layout.index(col).and_then(|idx| self.record.get(idx)) {
            Some(s) => self.ctx.transform(f(s)),
            None => Ok(default),
        }
    }
}
