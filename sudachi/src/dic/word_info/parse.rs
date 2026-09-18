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

use crate::dic::subset::InfoSubset;
use crate::dic::word_info::layout;
use crate::dic::word_info::{
    parse_i32_array, parse_u32_array, parse_user_data, WordInfoFixedData, WordInfoRawData,
};
use crate::error::SudachiResult;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SplitReadPlan {
    c_unit_split: bool,
    b_unit_split: bool,
    a_unit_split: bool,
    word_structure: bool,
}

impl SplitReadPlan {
    fn from_subset(subset: InfoSubset) -> Self {
        Self {
            c_unit_split: subset.intersects(
                InfoSubset::SPLIT_C
                    | InfoSubset::SPLIT_B
                    | InfoSubset::SPLIT_A
                    | InfoSubset::WORD_STRUCTURE,
            ),
            b_unit_split: subset
                .intersects(InfoSubset::SPLIT_B | InfoSubset::SPLIT_A | InfoSubset::WORD_STRUCTURE),
            a_unit_split: subset.intersects(InfoSubset::SPLIT_A | InfoSubset::WORD_STRUCTURE),
            word_structure: subset.contains(InfoSubset::WORD_STRUCTURE),
        }
    }
}

pub struct WordInfoParser {
    info: WordInfoRawData,
    flds: InfoSubset,
}

impl Default for WordInfoParser {
    #[inline]
    fn default() -> Self {
        Self::subset(InfoSubset::all())
    }
}

impl WordInfoParser {
    #[inline]
    pub fn subset(flds: InfoSubset) -> WordInfoParser {
        // normalize() adds the fixed fields required to interpret requested views.
        // For example, NORMALIZED_FORM/DICTIONARY_FORM require HEADWORD, and split
        // requests require INDEX_FORM_LENGTH.
        Self {
            info: Default::default(),
            flds: flds.normalize(),
        }
    }

    #[inline]
    pub fn embedded_c_unit_split_length(&self) -> usize {
        self.info.c_unit_split_length as usize
    }

    #[inline]
    pub fn embedded_b_unit_split_length(&self) -> usize {
        layout::embedded_len(self.info.b_unit_split_length)
    }

    #[inline]
    pub fn embedded_a_unit_split_length(&self) -> usize {
        layout::embedded_len(self.info.a_unit_split_length)
    }

    #[inline]
    pub fn embedded_word_structure_length(&self) -> usize {
        layout::embedded_len(self.info.word_structure_length)
    }

    #[inline]
    pub fn embedded_synonym_group_ids_length(&self) -> usize {
        self.info.synonym_group_ids_length as usize
    }

    #[inline]
    pub fn parse(mut self, data: &[u8]) -> SudachiResult<WordInfoRawData> {
        let (data, _) = nom::bytes::complete::take(layout::PARAMS_SIZE)(data)?;
        let (data, fixed) = WordInfoFixedData::parse(data)?;
        let split_plan = SplitReadPlan::from_subset(self.flds);
        self.copy_fixed_fields(&fixed);

        let (data, c_unit_split) = parse_u32_array(
            data,
            self.embedded_c_unit_split_length(),
            split_plan.c_unit_split,
        )?;
        if split_plan.c_unit_split {
            self.info.c_unit_split = c_unit_split;
        }

        let (data, b_unit_split) = parse_u32_array(
            data,
            self.embedded_b_unit_split_length(),
            split_plan.b_unit_split,
        )?;
        if fixed.b_unit_split_length < 0 {
            if split_plan.b_unit_split {
                self.info.b_unit_split = self.info.c_unit_split.clone();
            }
        } else if split_plan.b_unit_split {
            self.info.b_unit_split = b_unit_split;
        }

        let (data, a_unit_split) = parse_u32_array(
            data,
            self.embedded_a_unit_split_length(),
            split_plan.a_unit_split,
        )?;
        if fixed.a_unit_split_length < 0 {
            if split_plan.a_unit_split {
                self.info.a_unit_split = self.info.b_unit_split.clone();
            }
        } else if split_plan.a_unit_split {
            self.info.a_unit_split = a_unit_split;
        }

        let (data, word_structure) = parse_u32_array(
            data,
            self.embedded_word_structure_length(),
            split_plan.word_structure,
        )?;
        if fixed.word_structure_length < 0 {
            if split_plan.word_structure {
                self.info.word_structure = self.info.a_unit_split.clone();
            }
        } else if split_plan.word_structure {
            self.info.word_structure = word_structure;
        }

        let (data, synonym_group_ids) = parse_i32_array(
            data,
            self.embedded_synonym_group_ids_length(),
            self.keep_synonym_group_ids(),
        )?;
        if self.keep_synonym_group_ids() {
            self.info.synonym_group_ids = synonym_group_ids;
        }

        if fixed.has_user_data() {
            let (_, user_data) = parse_user_data(data, self.keep_user_data())?;
            if self.keep_user_data() {
                self.info.user_data = user_data;
            }
        }
        Ok(self.info)
    }

    fn copy_fixed_fields(&mut self, fixed: &WordInfoFixedData) {
        if self.flds.contains(InfoSubset::POS_ID) {
            self.info.pos_id = fixed.pos_id;
        }
        if self.flds.contains(InfoSubset::HEADWORD) {
            self.info.headword_strptr = fixed.headword_strptr;
        }
        if self.flds.contains(InfoSubset::READING_FORM) {
            self.info.reading_form_strptr = fixed.reading_form_strptr;
        }
        if self.flds.contains(InfoSubset::NORMALIZED_FORM) {
            self.info.normalized_form = fixed.normalized_form;
        }
        if self.flds.contains(InfoSubset::DICTIONARY_FORM) {
            self.info.dictionary_form = fixed.dictionary_form;
        }
        if self.flds.contains(InfoSubset::INDEX_FORM_LENGTH) {
            self.info.index_form_length = fixed.index_form_length;
        }
        self.info.c_unit_split_length = fixed.c_unit_split_length;
        self.info.b_unit_split_length = fixed.b_unit_split_length;
        self.info.a_unit_split_length = fixed.a_unit_split_length;
        self.info.word_structure_length = fixed.word_structure_length;
        self.info.synonym_group_ids_length = fixed.synonym_group_ids_length;
        self.info.user_data_flag = fixed.user_data_flag;
    }

    #[inline]
    fn keep_synonym_group_ids(&self) -> bool {
        self.flds.contains(InfoSubset::SYNONYM_GROUP_IDS)
    }

    #[inline]
    fn keep_user_data(&self) -> bool {
        self.flds.contains(InfoSubset::USER_DATA)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dic::lexicon::strings::StringPointer;

    fn push_u32s(buf: &mut Vec<u8>, data: &[u32]) {
        for value in data {
            buf.extend_from_slice(&value.to_le_bytes());
        }
    }

    fn push_i32s(buf: &mut Vec<u8>, data: &[i32]) {
        for value in data {
            buf.extend_from_slice(&value.to_le_bytes());
        }
    }

    fn push_utf16(buf: &mut Vec<u8>, data: &str) {
        let utf16: Vec<u16> = data.encode_utf16().collect();
        buf.extend_from_slice(&(utf16.len() as i16).to_le_bytes());
        for unit in utf16 {
            buf.extend_from_slice(&unit.to_le_bytes());
        }
    }

    #[test]
    fn parses_embedded_variable_length_fields() {
        let fixed = WordInfoFixedData {
            pos_id: 5,
            headword_strptr: StringPointer::unchecked(2, 4),
            reading_form_strptr: StringPointer::unchecked(3, 8),
            normalized_form: 11,
            dictionary_form: 12,
            index_form_length: 6,
            c_unit_split_length: 2,
            b_unit_split_length: 1,
            a_unit_split_length: 3,
            word_structure_length: 2,
            synonym_group_ids_length: 2,
            user_data_flag: 1,
        };

        let mut bytes = vec![0u8; layout::PARAMS_SIZE];
        fixed.write_to(&mut bytes).unwrap();
        push_u32s(&mut bytes, &[100, 101]);
        push_u32s(&mut bytes, &[200]);
        push_u32s(&mut bytes, &[300, 301, 302]);
        push_u32s(&mut bytes, &[400, 401]);
        push_i32s(&mut bytes, &[7, 8]);
        push_utf16(&mut bytes, "meta");

        let parsed = WordInfoParser::default().parse(&bytes).unwrap();
        assert_eq!(parsed.pos_id, fixed.pos_id);
        assert_eq!(parsed.c_unit_split, vec![100, 101]);
        assert_eq!(parsed.b_unit_split, vec![200]);
        assert_eq!(parsed.a_unit_split, vec![300, 301, 302]);
        assert_eq!(parsed.word_structure, vec![400, 401]);
        assert_eq!(parsed.synonym_group_ids, vec![7, 8]);
        assert_eq!(parsed.user_data, "meta");
    }

    #[test]
    fn expands_shared_split_arrays() {
        let fixed = WordInfoFixedData {
            pos_id: 9,
            headword_strptr: StringPointer::unchecked(1, 2),
            reading_form_strptr: StringPointer::unchecked(1, 4),
            normalized_form: 21,
            dictionary_form: 22,
            index_form_length: 3,
            c_unit_split_length: 2,
            b_unit_split_length: -1,
            a_unit_split_length: -1,
            word_structure_length: -1,
            synonym_group_ids_length: 1,
            user_data_flag: 0,
        };

        let mut bytes = vec![0u8; layout::PARAMS_SIZE];
        fixed.write_to(&mut bytes).unwrap();
        push_u32s(&mut bytes, &[10, 11]);
        push_i32s(&mut bytes, &[99]);

        let parsed = WordInfoParser::default().parse(&bytes).unwrap();
        assert_eq!(parsed.c_unit_split, vec![10, 11]);
        assert_eq!(parsed.b_unit_split, vec![10, 11]);
        assert_eq!(parsed.a_unit_split, vec![10, 11]);
        assert_eq!(parsed.word_structure, vec![10, 11]);
        assert_eq!(parsed.synonym_group_ids, vec![99]);
        assert!(parsed.user_data.is_empty());
    }

    #[test]
    fn subset_clears_unrequested_fields() {
        let fixed = WordInfoFixedData {
            pos_id: 9,
            headword_strptr: StringPointer::unchecked(1, 2),
            reading_form_strptr: StringPointer::unchecked(1, 4),
            normalized_form: 21,
            dictionary_form: 22,
            index_form_length: 3,
            c_unit_split_length: 2,
            b_unit_split_length: 1,
            a_unit_split_length: 1,
            word_structure_length: 1,
            synonym_group_ids_length: 1,
            user_data_flag: 1,
        };

        let mut bytes = vec![0u8; layout::PARAMS_SIZE];
        fixed.write_to(&mut bytes).unwrap();
        push_u32s(&mut bytes, &[10, 11]);
        push_u32s(&mut bytes, &[20]);
        push_u32s(&mut bytes, &[30]);
        push_u32s(&mut bytes, &[40]);
        push_i32s(&mut bytes, &[99]);
        push_utf16(&mut bytes, "meta");

        let parsed = WordInfoParser::subset(InfoSubset::READING_FORM)
            .parse(&bytes)
            .unwrap();
        assert_eq!(parsed.reading_form_strptr, fixed.reading_form_strptr);
        assert_eq!(parsed.pos_id, 0);
        assert_eq!(parsed.headword_strptr, Default::default());
        assert_eq!(parsed.normalized_form, 0);
        assert_eq!(parsed.dictionary_form, 0);
        assert_eq!(parsed.index_form_length, 0);
        assert!(parsed.c_unit_split.is_empty());
        assert!(parsed.b_unit_split.is_empty());
        assert!(parsed.a_unit_split.is_empty());
        assert!(parsed.word_structure.is_empty());
        assert!(parsed.synonym_group_ids.is_empty());
        assert!(parsed.user_data.is_empty());
    }

    #[test]
    fn subset_normalize_keeps_required_dependencies() {
        let fixed = WordInfoFixedData {
            pos_id: 7,
            headword_strptr: StringPointer::unchecked(2, 6),
            reading_form_strptr: StringPointer::unchecked(1, 2),
            normalized_form: 33,
            dictionary_form: 44,
            index_form_length: 5,
            c_unit_split_length: 1,
            b_unit_split_length: 0,
            a_unit_split_length: 0,
            word_structure_length: 0,
            synonym_group_ids_length: 0,
            user_data_flag: 0,
        };

        let mut bytes = vec![0u8; layout::PARAMS_SIZE];
        fixed.write_to(&mut bytes).unwrap();
        push_u32s(&mut bytes, &[10]);

        let parsed = WordInfoParser::subset(InfoSubset::NORMALIZED_FORM | InfoSubset::SPLIT_C)
            .parse(&bytes)
            .unwrap();
        assert_eq!(parsed.normalized_form, fixed.normalized_form);
        assert_eq!(parsed.headword_strptr, fixed.headword_strptr);
        assert_eq!(parsed.index_form_length, fixed.index_form_length);
        assert_eq!(parsed.c_unit_split, vec![10]);
    }
}
