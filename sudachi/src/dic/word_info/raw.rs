/*
 * Copyright (c) 2026 Works Applications Co., Ltd.
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

use crate::dic::lexicon::strings::StringPointer;

/// Parsed raw binary representation of a word info entry.
///
/// word id/ref fields are typed as u32 to avoid type conversion.
/// crate::dic::word_info::{WordInfoData, WordInfoRefData} will handle those types.
#[derive(Clone, Debug, Default)]
pub struct WordInfoRawData {
    pub pos_id: i16,

    pub headword_strptr: StringPointer,
    pub reading_form_strptr: StringPointer,
    pub normalized_form: u32,
    pub dictionary_form: u32,

    /// bytes length of the index form in utf-8.
    pub index_form_length: i16,
    pub c_unit_split_length: i8,
    pub b_unit_split_length: i8,
    pub a_unit_split_length: i8,
    pub word_structure_length: i8,
    pub synonym_group_ids_length: i8,
    pub user_data_flag: i8,

    pub c_unit_split: Vec<u32>,
    pub b_unit_split: Vec<u32>,
    pub a_unit_split: Vec<u32>,
    pub word_structure: Vec<u32>,
    pub synonym_group_ids: Vec<i32>,
    pub user_data: String,
}

#[derive(Clone, Debug, Default)]
pub struct WordInfoFixedData {
    pub pos_id: i16,
    pub headword_strptr: StringPointer,
    pub reading_form_strptr: StringPointer,
    pub normalized_form: u32,
    pub dictionary_form: u32,
    pub index_form_length: i16,
    pub c_unit_split_length: i8,
    pub b_unit_split_length: i8,
    pub a_unit_split_length: i8,
    pub word_structure_length: i8,
    pub synonym_group_ids_length: i8,
    pub user_data_flag: i8,
}

pub(crate) struct WordInfoVariableData<'a> {
    pub c_unit_split: &'a [u32],
    pub b_unit_split: &'a [u32],
    pub a_unit_split: &'a [u32],
    pub word_structure: &'a [u32],
    pub synonym_group_ids: &'a [i32],
    pub user_data: &'a str,
}
