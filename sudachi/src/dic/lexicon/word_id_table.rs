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

    pub fn entries(&self, index: usize) -> WordIdIter {
        debug_assert!(index < self.bytes.len());
        let ptr = unsafe { self.bytes.as_ptr().offset((index + self.offset) as isize) };
        let cnt = unsafe { ptr.read() } as usize;
        let data_ptr = unsafe { ptr.offset(1) } as *const u32;
        debug_assert!(index + cnt * std::mem::size_of::<u32>() + 1 <= self.bytes.len());
        WordIdIter {
            data: data_ptr,
            remaining: cnt,
        }
    }
}

pub struct WordIdIter {
    // this pointer is unaligned!
    data: *const u32,
    // number of remaining elements
    remaining: usize,
}

impl Iterator for WordIdIter {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }

        let val = unsafe { self.data.read_unaligned() };
        self.data = unsafe { self.data.offset(1) };
        self.remaining -= 1;
        Some(val)
    }
}

fn word_id_table_parser(
    input: &[u8],
    offset: usize,
    index: usize,
) -> SudachiNomResult<&[u8], Vec<u32>> {
    nom::sequence::preceded(take(offset + index), u32_array_parser)(input)
}
