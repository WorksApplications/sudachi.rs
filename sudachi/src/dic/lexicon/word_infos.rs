/*
 * Copyright (c) 2021 Works Applications Co., Ltd.
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

use crate::dic::lexicon_set::LexiconSet;
use crate::dic::read::u32_parser;
use crate::dic::read::word_info::WordInfoParser;
use crate::dic::subset::InfoSubset;
use crate::dic::word_id::WordId;
use crate::prelude::*;

pub struct WordInfos<'a> {
    bytes: &'a [u8],
    offset: usize,
    _word_size: u32,
    has_synonym_group_ids: bool,
}

impl<'a> WordInfos<'a> {
    pub fn new(
        bytes: &'a [u8],
        offset: usize,
        _word_size: u32,
        has_synonym_group_ids: bool,
    ) -> WordInfos {
        WordInfos {
            bytes,
            offset,
            _word_size,
            has_synonym_group_ids,
        }
    }

    fn word_id_to_offset(&self, word_id: u32) -> SudachiResult<usize> {
        Ok(u32_parser(&self.bytes[self.offset + (4 * word_id as usize)..])?.1 as usize)
    }

    fn parse_word_info(&self, word_id: u32, subset: InfoSubset) -> SudachiResult<WordInfoData> {
        let index = self.word_id_to_offset(word_id)?;
        let parser = WordInfoParser::subset(subset);
        parser.parse(&self.bytes[index..])
    }

    pub fn get_word_info(&self, word_id: u32, mut subset: InfoSubset) -> SudachiResult<WordInfo> {
        if !self.has_synonym_group_ids {
            subset -= InfoSubset::SYNONYM_GROUP_ID;
        }

        let mut word_info = self.parse_word_info(word_id, subset)?;

        // consult dictionary form
        let dfwi = word_info.dictionary_form_word_id;
        if (dfwi >= 0) && (dfwi != word_id as i32) {
            let inner = self.parse_word_info(dfwi as u32, InfoSubset::SURFACE)?;
            word_info.dictionary_form = inner.surface;
        };

        Ok(word_info.into())
    }
}

/// Internal storage of the WordInfo.
/// It is not accessible by default, but a WordInfo can be created from it:
/// `let wi: WordInfo = data.into();`
///
/// String fields CAN be empty, in this case the value of the surface field should be used instead
#[derive(Clone, Debug, Default)]
pub struct WordInfoData {
    pub surface: String,
    pub head_word_length: u16,
    pub pos_id: u16,
    pub normalized_form: String,
    pub dictionary_form_word_id: i32,
    pub dictionary_form: String,
    pub reading_form: String,
    pub a_unit_split: Vec<WordId>,
    pub b_unit_split: Vec<WordId>,
    pub word_structure: Vec<WordId>,
    pub synonym_group_ids: Vec<u32>,
}

/// WordInfo API.
///
/// Internal data is not accessible by default, but can be extracted as
/// `let data: WordInfoData = info.into()`.
/// Note: this will consume WordInfo.
#[derive(Clone, Default)]
#[repr(transparent)]
pub struct WordInfo {
    data: WordInfoData,
}

impl WordInfo {
    pub fn surface(&self) -> &str {
        &self.data.surface
    }

    pub fn head_word_length(&self) -> usize {
        self.data.head_word_length as usize
    }

    pub fn pos_id(&self) -> u16 {
        self.data.pos_id
    }

    pub fn normalized_form(&self) -> &str {
        if self.data.normalized_form.is_empty() {
            self.surface()
        } else {
            &self.data.normalized_form
        }
    }

    pub fn dictionary_form_word_id(&self) -> i32 {
        self.data.dictionary_form_word_id
    }

    pub fn dictionary_form(&self) -> &str {
        if self.data.dictionary_form.is_empty() {
            self.surface()
        } else {
            &self.data.dictionary_form
        }
    }

    pub fn reading_form(&self) -> &str {
        if self.data.reading_form.is_empty() {
            self.surface()
        } else {
            &self.data.reading_form
        }
    }

    pub fn a_unit_split(&self) -> &[WordId] {
        &self.data.a_unit_split
    }

    pub fn b_unit_split(&self) -> &[WordId] {
        &self.data.b_unit_split
    }

    pub fn word_structure(&self) -> &[WordId] {
        &self.data.word_structure
    }

    pub fn synonym_group_ids(&self) -> &[u32] {
        &self.data.synonym_group_ids
    }

    pub fn borrow_data(&self) -> &WordInfoData {
        &self.data
    }
}

impl From<WordInfoData> for WordInfo {
    fn from(data: WordInfoData) -> Self {
        WordInfo { data }
    }
}

impl From<WordInfo> for WordInfoData {
    fn from(info: WordInfo) -> Self {
        info.data
    }
}

struct SplitIter<'a> {
    index: usize,
    split: &'a [WordId],
    lexicon: &'a LexiconSet<'a>,
}

impl Iterator for SplitIter<'_> {
    type Item = SudachiResult<WordInfo>;

    fn next(&mut self) -> Option<Self::Item> {
        let idx = self.index;
        if idx >= self.split.len() {
            None
        } else {
            self.index += 1;
            Some(self.lexicon.get_word_info(self.split[idx]))
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let rem = self.split.len() - self.index;
        (rem, Some(rem))
    }
}

impl FusedIterator for SplitIter<'_> {}
