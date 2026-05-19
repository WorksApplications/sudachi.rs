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

use std::iter::FusedIterator;

use crate::dic::subset::InfoSubset;
use crate::dic::word_id::EntryId;
use crate::dic::word_info::layout;
use crate::dic::word_info::{WordInfoFixedData, WordInfoParser, WordInfoRefData};
use crate::prelude::*;
use thiserror::Error;

/// Errors returned while scanning binary WordInfo entry boundaries.
#[derive(Error, Debug, Clone, Eq, PartialEq)]
#[non_exhaustive]
pub enum WordInfoError {
    /// WordInfo entry offsets must be aligned because entry ids are derived from them.
    #[error("word info entry id at byte offset {0} is not aligned")]
    EntryIdNotAligned(usize),

    /// The scanner could not determine a complete, valid entry size at the offset.
    #[error("failed to load word info entry size at byte offset {0}")]
    FailedToLoadEntrySize(usize),

    /// The entry offset cannot be represented as a valid Sudachi entry id.
    #[error("word info entry id at byte offset {0} is too large")]
    EntryIdTooLarge(usize),
}

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
        self.entry_ids(num_total_entries)
            .collect::<SudachiResult<Vec<_>>>()
            .ok()
    }

    pub(crate) fn entry_ids(&self, num_total_entries: u32) -> WordInfoEntryIdIter<'_, '_> {
        WordInfoEntryIdIter {
            infos: self,
            cursor: Self::entry_id_cursor(num_total_entries),
        }
    }

    pub(crate) fn entry_id_cursor(num_total_entries: u32) -> WordInfoEntryIdCursor {
        WordInfoEntryIdCursor {
            remaining: num_total_entries,
            offset: Self::ENTRIES_INITIAL_OFFSET,
        }
    }

    /// Validate that all WordInfo entries can be scanned from their binary boundaries.
    ///
    /// Returns [`WordInfoError`] through [`SudachiError`] when an entry id cannot be
    /// derived or an entry size cannot be read.
    pub fn validate_entry_boundaries(&self, num_total_entries: u32) -> SudachiResult<()> {
        let mut cursor = Self::entry_id_cursor(num_total_entries);
        while self.next_entry_id(&mut cursor)?.is_some() {}
        Ok(())
    }

    pub(crate) fn next_entry_id(
        &self,
        cursor: &mut WordInfoEntryIdCursor,
    ) -> SudachiResult<Option<EntryId>> {
        if cursor.remaining == 0 {
            return Ok(None);
        }

        let entry_id = Self::entry_id_from_offset(cursor.offset)?;
        let size = self
            .entry_size_at(cursor.offset)
            .ok_or_else(|| WordInfoError::FailedToLoadEntrySize(cursor.offset))?;

        cursor.offset = match cursor.offset.checked_add(size) {
            Some(offset) => offset,
            None => return Err(WordInfoError::EntryIdTooLarge(cursor.offset).into()),
        };
        cursor.remaining -= 1;
        Ok(Some(entry_id))
    }

    fn entry_id_from_offset(offset: usize) -> SudachiResult<EntryId> {
        if offset % Self::WORD_INFO_OFFSET_ALIGNMENT != 0 {
            return Err(WordInfoError::EntryIdNotAligned(offset).into());
        }

        let raw = offset >> Self::WORD_ID_ALIGNMENT_BITS;
        if raw > EntryId::MAX as usize {
            return Err(WordInfoError::EntryIdTooLarge(offset).into());
        }

        Ok(EntryId::new(raw as u32))
    }

    fn entry_size_at(&self, offset: usize) -> Option<usize> {
        let entry_bytes = self.bytes.get(offset..)?;
        let fixed = WordInfoFixedData::from_entry_bytes(entry_bytes)?;

        if !layout::is_valid_user_data_flag(fixed.user_data_flag) {
            return None;
        }

        let mut user_data_units = None;
        if fixed.has_user_data() {
            let user_data_offset = offset.checked_add(layout::unaligned_size_from_lengths(
                fixed.c_unit_split_length,
                fixed.b_unit_split_length,
                fixed.a_unit_split_length,
                fixed.word_structure_length,
                fixed.synonym_group_ids_length,
                None,
            )?)?;
            let user_data_len_end = user_data_offset.checked_add(2)?;
            let user_len_bytes = self.bytes.get(user_data_offset..user_data_len_end)?;
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
        let end = offset.checked_add(aligned)?;
        self.bytes.get(offset..end)?;
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

pub(crate) struct WordInfoEntryIdCursor {
    remaining: u32,
    offset: usize,
}

pub(crate) struct WordInfoEntryIdIter<'a, 'b> {
    infos: &'a WordInfos<'b>,
    cursor: WordInfoEntryIdCursor,
}

impl Iterator for WordInfoEntryIdIter<'_, '_> {
    type Item = SudachiResult<EntryId>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.infos.next_entry_id(&mut self.cursor) {
            Ok(Some(entry_id)) => Some(Ok(entry_id)),
            Ok(None) => None,
            Err(error) => {
                self.cursor.remaining = 0;
                Some(Err(error))
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.cursor.remaining as usize;
        (0, Some(remaining))
    }
}

impl FusedIterator for WordInfoEntryIdIter<'_, '_> {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dic::lexicon::strings::StringPointer;
    use crate::dic::word_id::DictId;
    use crate::dic::word_info::WordInfoVariableData;

    fn assert_word_info_error(error: SudachiError, expected: WordInfoError) {
        match error {
            SudachiError::WordInfo(actual) => assert_eq!(actual, expected),
            other => panic!("expected word info error {expected:?}, got {other:?}"),
        }
    }

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

    #[test]
    fn validate_entry_boundaries_rejects_malformed_entries() {
        let mut bytes = make_entry(&sample_fixed());
        bytes.truncate(bytes.len() - 1);
        let infos = WordInfos::from_bytes(&bytes);

        let err = infos.validate_entry_boundaries(1).unwrap_err();
        assert_word_info_error(
            err,
            WordInfoError::FailedToLoadEntrySize(layout::ENTRY_INITIAL_OFFSET),
        );
    }

    #[test]
    fn validate_entry_boundaries_rejects_short_entry_block() {
        let bytes = vec![0; layout::ENTRY_INITIAL_OFFSET - 1];
        let infos = WordInfos::from_bytes(&bytes);

        let err = infos.validate_entry_boundaries(1).unwrap_err();
        assert_word_info_error(
            err,
            WordInfoError::FailedToLoadEntrySize(layout::ENTRY_INITIAL_OFFSET),
        );
    }

    #[test]
    fn next_entry_id_rejects_misaligned_entry_offset() {
        let bytes = make_entry(&sample_fixed());
        let infos = WordInfos::from_bytes(&bytes);
        let mut cursor = WordInfoEntryIdCursor {
            remaining: 1,
            offset: layout::ENTRY_INITIAL_OFFSET + 1,
        };

        let err = infos.next_entry_id(&mut cursor).unwrap_err();
        assert_word_info_error(
            err,
            WordInfoError::EntryIdNotAligned(layout::ENTRY_INITIAL_OFFSET + 1),
        );
    }

    #[test]
    fn next_entry_id_rejects_too_large_entry_id() {
        let bytes = make_entry(&sample_fixed());
        let infos = WordInfos::from_bytes(&bytes);
        let too_large_offset = ((EntryId::MAX as usize) + 1) << WordInfos::WORD_ID_ALIGNMENT_BITS;
        let mut cursor = WordInfoEntryIdCursor {
            remaining: 1,
            offset: too_large_offset,
        };

        let err = infos.next_entry_id(&mut cursor).unwrap_err();
        assert_word_info_error(err, WordInfoError::EntryIdTooLarge(too_large_offset));
    }

    #[test]
    fn entry_id_iterator_reports_malformed_entries() {
        let mut bytes = make_entry(&sample_fixed());
        bytes.truncate(bytes.len() - 1);
        let infos = WordInfos::from_bytes(&bytes);
        let mut entries = infos.entry_ids(1);

        assert!(entries.next().unwrap().is_err());
        assert!(entries.next().is_none());
    }
}
