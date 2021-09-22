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

use nom::bytes::complete::take;

use crate::dic::u32_array_parser;
use crate::error::SudachiNomResult;
use crate::prelude::*;

pub struct WordIdTable<'a> {
    bytes: &'a [u8],
    size: u32,
    offset: usize,
}

impl<'a> WordIdTable<'a> {
    pub fn new(bytes: &'a [u8], size: u32, offset: usize) -> WordIdTable {
        WordIdTable {
            bytes,
            size,
            offset,
        }
    }

    pub fn storage_size(&self) -> usize {
        4 + self.size as usize
    }

    pub fn get(&self, index: usize) -> SudachiResult<Vec<u32>> {
        let (_rest, result) = word_id_table_parser(self.bytes, self.offset, index)?;
        Ok(result)
    }
}

fn word_id_table_parser(
    input: &[u8],
    offset: usize,
    index: usize,
) -> SudachiNomResult<&[u8], Vec<u32>> {
    nom::sequence::preceded(take(offset + index), u32_array_parser)(input)
}
