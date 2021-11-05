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

use nom::{
    bytes::complete::take,
    number::complete::{le_i32, le_u16, le_u32},
};

use crate::dic::lexicon_set::LexiconSet;
use crate::dic::read::{
    string_length_parser, u32_array_parser, u32_wid_array_parser, utf16_string_parser,
};
use crate::dic::word_id::WordId;
use crate::error::SudachiNomResult;
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

    pub fn get_word_info(&self, word_id: u32) -> SudachiResult<WordInfo> {
        let index = self.word_id_to_offset(word_id)?;
        let mut word_info = word_info_parser(self.bytes, index, self.has_synonym_group_ids)?.1;

        // consult dictionary form
        let dfwi = word_info.dictionary_form_word_id;
        if (dfwi >= 0) && (dfwi != word_id as i32) {
            word_info.dictionary_form = self.get_word_info(dfwi as u32)?.surface;
        };

        Ok(word_info)
    }
}

fn u32_parser(input: &[u8]) -> SudachiNomResult<&[u8], u32> {
    le_u32(input)
}

pub(crate) fn word_info_parser(
    input: &[u8],
    offset: usize,
    has_synonym_group_ids: bool,
) -> SudachiNomResult<&[u8], WordInfo> {
    let (
        rest,
        (
            surface,
            head_word_length,
            pos_id,
            normalized_form,
            dictionary_form_word_id,
            reading_form,
            a_unit_split,
            b_unit_split,
            word_structure,
            synonym_group_ids,
        ),
    ) = nom::sequence::preceded(
        take(offset),
        nom::sequence::tuple((
            utf16_string_parser,
            string_length_parser,
            le_u16,
            utf16_string_parser,
            le_i32,
            utf16_string_parser,
            u32_wid_array_parser,
            u32_wid_array_parser,
            u32_wid_array_parser,
            nom::combinator::cond(has_synonym_group_ids, u32_array_parser),
        )),
    )(input)?;

    Ok((
        rest,
        WordInfo {
            head_word_length,
            pos_id,
            normalized_form: match normalized_form.len() {
                0 => surface.clone(),
                _ => normalized_form,
            },
            dictionary_form_word_id,
            dictionary_form: surface.clone(),
            surface, // after normalized_form and dictionary_form, as it may be cloned there
            reading_form,
            a_unit_split,
            b_unit_split,
            word_structure,
            synonym_group_ids: synonym_group_ids.unwrap_or_else(|| Vec::new()),
        },
    ))
}

#[derive(Clone, Debug, Default)]
pub struct WordInfo {
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
}

impl FusedIterator for SplitIter<'_> {}
