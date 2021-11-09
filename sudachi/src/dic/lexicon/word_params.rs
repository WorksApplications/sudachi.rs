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

use crate::util::cow_array::CowArray;

pub struct WordParams<'a> {
    data: CowArray<'a, i16>,
    size: u32,
}

impl<'a> WordParams<'a> {
    const PARAM_SIZE: usize = 3;
    const ELEMENT_SIZE: usize = 2 * Self::PARAM_SIZE;

    pub fn new(bytes: &'a [u8], size: u32, offset: usize) -> WordParams {
        let n_entries = size as usize * Self::PARAM_SIZE;
        Self {
            data: CowArray::from_bytes(bytes, offset, n_entries),
            size,
        }
    }

    pub fn storage_size(&self) -> usize {
        4 + WordParams::ELEMENT_SIZE * self.size as usize
    }

    pub fn size(&self) -> u32 {
        self.size
    }

    #[inline]
    pub fn get_params(&self, word_id: u32) -> (i16, i16, i16) {
        let begin = word_id as usize * Self::PARAM_SIZE;
        let end = begin + Self::PARAM_SIZE;
        let slice = &self.data[begin..end];
        (slice[0], slice[1], slice[2])
    }

    pub fn get_cost(&self, word_id: u32) -> i16 {
        let cost_offset = word_id as usize * Self::PARAM_SIZE + 2;
        self.data[cost_offset]
    }

    pub fn set_cost(&mut self, word_id: u32, cost: i16) {
        let cost_offset = word_id as usize * Self::PARAM_SIZE + 2;
        self.data.set(cost_offset, cost)
    }
}
