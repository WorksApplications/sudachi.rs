/*
 *  Copyright (c) 2021-2026 Works Applications Co., Ltd.
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

use nom::number::complete::le_i16;

use crate::error::{SudachiError, SudachiResult};
use crate::util::cow_array::CowArray;

pub struct ConnectionMatrix<'a> {
    data: CowArray<'a, i16>,
    num_left: usize,
    num_right: usize,
}

impl<'a> ConnectionMatrix<'a> {
    pub fn from_bytes(buf: &'a [u8]) -> SudachiResult<ConnectionMatrix<'a>> {
        let (rest, (num_left, num_right)) = nom::sequence::tuple((le_i16, le_i16))(buf)?;
        Self::from_offset_size(rest, 0, num_left as usize, num_right as usize)
    }

    pub fn from_offset_size(
        data: &'a [u8],
        offset: usize,
        num_left: usize,
        num_right: usize,
    ) -> SudachiResult<ConnectionMatrix<'a>> {
        let size = num_left * num_right;
        let end = offset + size * std::mem::size_of::<i16>();
        if end > data.len() {
            return Err(SudachiError::InvalidDictionaryGrammar.with_context("connection matrix"));
        }

        Ok(ConnectionMatrix {
            data: CowArray::from_bytes(data, offset, size),
            num_left,
            num_right,
        })
    }

    #[inline(always)]
    fn index(&self, left: u16, right: u16) -> usize {
        let uleft = left as usize;
        let uright = right as usize;
        debug_assert!(
            uleft < self.num_left,
            "left id {} is out of range (num_left={})",
            uleft,
            self.num_left
        );
        debug_assert!(
            uright < self.num_right,
            "right id {} is out of range (num_right={})",
            uright,
            self.num_right
        );
        let index = uright * self.num_left + uleft;
        debug_assert!(index < self.data.len());
        index
    }

    /// Gets the value of the connection matrix
    ///
    /// It is performance critical that this function
    /// 1. Has no branches
    /// 2. Is inlined to the caller
    ///
    /// This is UB if index is out of bounds, but that can't happen
    /// except in the case if the binary dictionary was tampered with.
    /// It is OK to make usage of tampered binary dictionaries UB.
    #[inline(always)]
    pub fn cost(&self, left: u16, right: u16) -> i16 {
        let index = self.index(left, right);
        *unsafe { self.data.get_unchecked(index) }
    }

    pub fn update(&mut self, left: u16, right: u16, value: i16) {
        let index = self.index(left, right);
        self.data.set(index, value);
    }

    /// Returns maximum number of left connection ID
    pub fn num_left(&self) -> usize {
        self.num_left
    }

    /// Returns maximum number of right connection ID
    pub fn num_right(&self) -> usize {
        self.num_right
    }
}

impl ConnectionMatrix<'static> {
    pub fn empty() -> Self {
        ConnectionMatrix {
            data: CowArray::from_owned(Vec::new()),
            num_left: 0,
            num_right: 0,
        }
    }
}
