/*
 * Copyright (c) 2021-2026 Works Applications Co., Ltd.
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use crate::dic::subset::InfoSubset;
use crate::dic::word_id::EntryId;
use crate::dic::word_info::layout;
use crate::dic::word_info::{WordInfoFixedData, WordInfoParser, WordInfoRefData};
use crate::prelude::*;

pub struct WordInfos<'a> {
    bytes: &'a [u8],
}

impl<'a> WordInfos<'a> {
    pub const ENTRIES_INITIAL_OFFSET: usize = layout::ENTRY_INITIAL_OFFSET;
    pub const WORD_ID_ALIGNMENT_BITS: usize = layout::WORD_ID_ALIGNMENT_BITS;
    pub const WORD_INFO_OFFSET_ALIGNMENT: usize = layout::WORD_INFO_OFFSET_ALIGNMENT;

    pub fn from_bytes(bytes: &'a [u8]) -> WordInfos<'a> {
        WordInfos { bytes }
    }

    pub fn entry_id_to_offset(entry_id: EntryId) -> usize {
        (entry_id.as_raw() as usize) << Self::WORD_ID_ALIGNMENT_BITS
    }

    pub fn entry_ids_in_order(&self, num_total_entries: u32) -> Option<Vec<EntryId>> {
        let mut result = Vec::with_capacity(num_total_entries as usize);
        let mut offset = Self::ENTRIES_INITIAL_OFFSET;

        while result.len() < num_total_entries as usize {
            if offset % Self::WORD_INFO_OFFSET_ALIGNMENT != 0 {
                return None;
            }

            let entry_id = EntryId::new((offset >> Self::WORD_ID_ALIGNMENT_BITS) as u32);
            result.push(entry_id);

            let size = self.entry_size_at(offset)?;
            offset = offset.checked_add(size)?;
        }

        Some(result)
    }

    fn entry_size_at(&self, offset: usize) -> Option<usize> {
        let fixed = WordInfoFixedData::from_entry_bytes(&self.bytes[offset..])?;

        if !layout::is_valid_user_data_flag(fixed.user_data_flag) {
            return None;
        }

        let mut user_data_units = None;
        if fixed.has_user_data() {
            let user_data_offset = offset
                + layout::unaligned_size_from_lengths(
                    fixed.c_unit_split_length,
                    fixed.b_unit_split_length,
                    fixed.a_unit_split_length,
                    fixed.word_structure_length,
                    fixed.synonym_group_ids_length,
                    None,
                )?;
            let user_len_bytes = self.bytes.get(user_data_offset..user_data_offset + 2)?;
            let user_len = i16::from_le_bytes([user_len_bytes[0], user_len_bytes[1]]);
            user_data_units = Some(user_len);
        }

        let aligned = layout::size_from_lengths(
            fixed.c_unit_split_length,
            fixed.b_unit_split_length,
            fixed.a_unit_split_length,
            fixed.word_structure_length,
            fixed.synonym_group_ids_length,
            user_data_units,
        )?;
        self.bytes.get(offset..offset + aligned)?;
        Some(aligned)
    }

    pub fn get_word_info(
        &self,
        entry_id: EntryId,
        subset: InfoSubset,
    ) -> SudachiResult<WordInfoRefData> {
        let offset = Self::entry_id_to_offset(entry_id);
        let parser = WordInfoParser::subset(subset);
        let bytes = self.bytes.get(offset..).ok_or_else(|| {
            SudachiError::InvalidDataFormat(
                0,
                format!("invalid word info entry id: {}", entry_id.as_raw()),
            )
        })?;
        let word_info = parser.parse(bytes)?;
        Ok(WordInfoRefData::from_raw(word_info))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dic::lexicon::strings::StringPointer;
    use crate::dic::word_id::DictId;
    use crate::dic::word_info::WordInfoVariableData;

    fn sample_fixed() -> WordInfoFixedData {
        WordInfoFixedData {
            pos_id: 3,
            headword_strptr: StringPointer::unchecked(2, 4),
            reading_form_strptr: StringPointer::unchecked(3, 8),
            normalized_form: 10,
            dictionary_form: 11,
            index_form_length: 6,
            c_unit_split_length: 2,
            b_unit_split_length: -1,
            a_unit_split_length: 1,
            word_structure_length: -1,
            synonym_group_ids_length: 2,
            user_data_flag: 1,
        }
    }

    fn make_entry(fixed: &WordInfoFixedData) -> Vec<u8> {
        let variable = WordInfoVariableData {
            c_unit_split: &[100, 101],
            b_unit_split: &[100, 101],
            a_unit_split: &[200],
            word_structure: &[200],
            synonym_group_ids: &[7, 8],
            user_data: "meta",
        };
        let mut bytes = vec![0u8; layout::ENTRY_INITIAL_OFFSET + layout::PARAMS_SIZE];
        fixed.write_to(&mut bytes).unwrap();
        variable.write_to(&mut bytes, fixed).unwrap();
        let aligned = layout::aligned_size(bytes.len());
        bytes.resize(aligned, 0);
        bytes
    }

    #[test]
    fn rejects_invalid_user_data_flag() {
        let mut fixed = sample_fixed();
        fixed.user_data_flag = 2;
        let bytes = make_entry(&fixed);
        let infos = WordInfos::from_bytes(&bytes);
        assert!(infos.entry_size_at(layout::ENTRY_INITIAL_OFFSET).is_none());
    }

    #[test]
    fn rejects_truncated_user_data_length() {
        let fixed = sample_fixed();
        let mut bytes = make_entry(&fixed);
        let user_len_offset = layout::ENTRY_INITIAL_OFFSET
            + layout::unaligned_size_from_lengths(
                fixed.c_unit_split_length,
                fixed.b_unit_split_length,
                fixed.a_unit_split_length,
                fixed.word_structure_length,
                fixed.synonym_group_ids_length,
                None,
            )
            .unwrap();
        bytes.truncate(user_len_offset + 1);
        let infos = WordInfos::from_bytes(&bytes);
        assert!(infos.entry_size_at(layout::ENTRY_INITIAL_OFFSET).is_none());
    }

    #[test]
    fn rejects_split_payload_shorter_than_length() {
        let fixed = WordInfoFixedData {
            user_data_flag: 0,
            synonym_group_ids_length: 0,
            word_structure_length: 0,
            a_unit_split_length: 0,
            b_unit_split_length: 0,
            c_unit_split_length: 2,
            ..sample_fixed()
        };
        let mut bytes = vec![0u8; layout::ENTRY_INITIAL_OFFSET + layout::PARAMS_SIZE];
        fixed.write_to(&mut bytes).unwrap();
        bytes.extend_from_slice(&10u32.to_le_bytes());
        let infos = WordInfos::from_bytes(&bytes);
        assert!(infos.entry_size_at(layout::ENTRY_INITIAL_OFFSET).is_none());
    }

    #[test]
    fn parser_and_scanner_agree_on_entry_boundaries() {
        let first = make_entry(&sample_fixed());
        let second_fixed = WordInfoFixedData {
            pos_id: 9,
            headword_strptr: StringPointer::unchecked(1, 2),
            reading_form_strptr: StringPointer::unchecked(1, 4),
            normalized_form: 21,
            dictionary_form: 22,
            index_form_length: 3,
            c_unit_split_length: 1,
            b_unit_split_length: 0,
            a_unit_split_length: 0,
            word_structure_length: 0,
            synonym_group_ids_length: 0,
            user_data_flag: 0,
        };
        let mut second = vec![0u8; layout::PARAMS_SIZE];
        second_fixed.write_to(&mut second).unwrap();
        second.extend_from_slice(&55u32.to_le_bytes());
        second.resize(layout::aligned_size(second.len()), 0);

        let mut bytes = first.clone();
        bytes.extend_from_slice(&second);

        let infos = WordInfos::from_bytes(&bytes);
        let ids = infos.entry_ids_in_order(2).unwrap();
        assert_eq!(ids[0], EntryId::new(4));
        let second_offset = WordInfos::entry_id_to_offset(ids[1]);
        assert_eq!(second_offset, first.len());

        let first_info = infos.get_word_info(ids[0], InfoSubset::all()).unwrap();
        let second_info = infos.get_word_info(ids[1], InfoSubset::all()).unwrap();
        assert_eq!(
            first_info
                .resolve(DictId::SYSTEM, 0, &[0], InfoSubset::all())
                .index_form_length(),
            6
        );
        assert_eq!(
            second_info
                .resolve(DictId::SYSTEM, 0, &[0], InfoSubset::all())
                .c_unit_split()
                .len(),
            1
        );
    }
}
