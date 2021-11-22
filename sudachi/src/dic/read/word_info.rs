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

use crate::dic::lexicon::word_infos::WordInfoData;
use crate::dic::read::u16str::*;
use crate::dic::read::{skip_u32_array, skip_wid_array, u32_array_parser, u32_wid_array_parser};
use crate::dic::subset::InfoSubset;
use crate::error::SudachiResult;
use nom::number::complete::{le_i32, le_u16};

pub struct WordInfoParser {
    info: WordInfoData,
    flds: InfoSubset,
}

/// Parse a single field of the WordInfo binary representation.
/// Six-parameter version accepts two funcitons:
/// true function which will actually parse the data, and
/// false function which should skip reading the data and just advance the parser position
///
/// Five-parameter version accepts only a single function and will unconditionally write
/// value from the binary form into the structure.
///
/// Six-parameter version should be used for "heavy" fields which require memory allocation
/// and five-parameter version should be used for "light" fields.
macro_rules! parse_field {
    ($root: expr, $data: ident, $name:tt, $field:expr, $tfn:tt, $ffn:tt) => {
        if $root.flds.is_empty() {
            return Ok($root.info);
        }
        #[allow(unused)]
        let $data = if $root.flds.contains($field) {
            let (next, res) = $tfn($data)?;
            $root.info.$name = res;
            $root.flds -= $field;
            next
        } else {
            let (next, _) = $ffn($data)?;
            next
        };
    };
    ($root: expr, $data: ident, $name:tt, $field:expr, $tfn:tt) => {
        if $root.flds.is_empty() {
            return Ok($root.info);
        }
        $root.flds -= $field;
        #[allow(unused)]
        let $data = {
            let (next, res) = $tfn($data)?;
            $root.info.$name = res;
            next
        };
    };
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
        Self {
            info: Default::default(),
            flds,
        }
    }

    #[inline]
    pub fn parse(mut self, data: &[u8]) -> SudachiResult<WordInfoData> {
        parse_field!(
            self,
            data,
            surface,
            InfoSubset::SURFACE,
            utf16_string_parser,
            skip_u16_string
        );
        parse_field!(
            self,
            data,
            head_word_length,
            InfoSubset::HEAD_WORD_LENGTH,
            string_length_parser
        );
        parse_field!(self, data, pos_id, InfoSubset::POS_ID, le_u16);
        parse_field!(
            self,
            data,
            normalized_form,
            InfoSubset::NORMALIZED_FORM,
            utf16_string_parser,
            skip_u16_string
        );
        parse_field!(
            self,
            data,
            dictionary_form_word_id,
            InfoSubset::DIC_FORM_WORD_ID,
            le_i32
        );
        parse_field!(
            self,
            data,
            reading_form,
            InfoSubset::READING_FORM,
            utf16_string_parser,
            skip_u16_string
        );
        parse_field!(
            self,
            data,
            a_unit_split,
            InfoSubset::SPLIT_A,
            u32_wid_array_parser,
            skip_wid_array
        );
        parse_field!(
            self,
            data,
            b_unit_split,
            InfoSubset::SPLIT_B,
            u32_wid_array_parser,
            skip_wid_array
        );
        parse_field!(
            self,
            data,
            word_structure,
            InfoSubset::WORD_STRUCTURE,
            u32_wid_array_parser,
            skip_wid_array
        );
        parse_field!(
            self,
            data,
            synonym_group_ids,
            InfoSubset::SYNONYM_GROUP_ID,
            u32_array_parser,
            skip_u32_array
        );
        Ok(self.info)
    }
}
