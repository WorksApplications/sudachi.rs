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

use nom::{bytes::complete::take, number::complete::le_i16};
use std::collections::HashMap;

use crate::error::SudachiNomResult;
use crate::prelude::*;

pub struct WordParams<'a> {
    bytes: &'a [u8],
    size: u32,
    offset: usize,
    cost_map: HashMap<u32, i16>,
}

impl<'a> WordParams<'a> {
    const ELEMENT_SIZE: usize = 2 * 3;

    pub fn new(bytes: &'a [u8], size: u32, offset: usize) -> WordParams {
        WordParams {
            bytes,
            size,
            offset,
            cost_map: HashMap::new(),
        }
    }

    pub fn storage_size(&self) -> usize {
        4 + WordParams::ELEMENT_SIZE * self.size as usize
    }

    pub fn size(&self) -> u32 {
        self.size
    }

    pub fn get_left_id(&self, word_id: u32) -> SudachiResult<i16> {
        let (_rest, num) = i16_parser(
            self.bytes,
            self.offset + WordParams::ELEMENT_SIZE * word_id as usize,
        )?;
        Ok(num)
    }

    pub fn get_right_id(&self, word_id: u32) -> SudachiResult<i16> {
        let (_rest, num) = i16_parser(
            self.bytes,
            self.offset + WordParams::ELEMENT_SIZE * word_id as usize + 2,
        )?;
        Ok(num)
    }

    pub fn get_cost(&self, word_id: u32) -> SudachiResult<i16> {
        if let Some(v) = self.cost_map.get(&word_id) {
            return Ok(*v);
        }

        let (_rest, num) = i16_parser(
            self.bytes,
            self.offset + WordParams::ELEMENT_SIZE * word_id as usize + 4,
        )?;
        Ok(num)
    }

    pub fn set_cost(&mut self, word_id: u32, cost: i16) {
        self.cost_map.insert(word_id, cost);
    }
}

fn i16_parser(input: &[u8], offset: usize) -> SudachiNomResult<&[u8], i16> {
    nom::sequence::preceded(take(offset), le_i16)(input)
}
