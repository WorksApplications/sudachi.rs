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

use crate::error::{SudachiError, SudachiResult};

pub struct ConnectionMatrix<'a> {
    /// This is for storage only and can be unused
    #[allow(unused)]
    storage: Option<Vec<i16>>,
    array: &'a [i16],
    num_left: usize,
    num_right: usize,
}

fn is_aligned(num: usize, alignment: usize) -> bool {
    debug_assert!(alignment.is_power_of_two());
    let mask = alignment - 1;
    num & mask == 0
}

impl<'a> ConnectionMatrix<'a> {
    pub fn from_offset_size(
        data: &'a [u8],
        offset: usize,
        num_left: usize,
        num_right: usize,
    ) -> SudachiResult<ConnectionMatrix<'a>> {
        let size = num_left * num_right;

        let end = offset + size;
        if end > data.len() {
            return Err(SudachiError::InvalidDictionaryGrammar);
        }

        let (storage, slice) = if is_aligned(offset, std::mem::align_of::<i16>()) {
            let ptr = unsafe { data.as_ptr().offset(offset as isize) as *const i16 };
            let slice: &'a [i16] = unsafe { &*std::ptr::slice_from_raw_parts(ptr, size) };
            (None, slice)
        } else {
            let data = Self::copy_of(&data[offset..offset + size], size);
            let slice: &'a [i16] = unsafe { std::mem::transmute(data.as_slice()) };
            (Some(data), slice)
        };

        Ok(ConnectionMatrix {
            storage,
            array: slice,
            num_left,
            num_right,
        })
    }

    // data is not aligned for i16
    fn copy_of(data: &[u8], size: usize) -> Vec<i16> {
        let mut result = Vec::with_capacity(size);
        let data = data.as_ptr() as *const i16;
        for i in 0..size as isize {
            result.push(unsafe { data.offset(i).read_unaligned() });
        }
        result
    }

    #[inline(always)]
    fn index(&self, left: u16, right: u16) -> usize {
        let uleft = left as usize;
        let uright = right as usize;
        debug_assert!(uleft < self.num_left);
        debug_assert!(uright < self.num_right);
        let index = uright * self.num_left + uleft;
        debug_assert!(index < self.array.len());
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
        *unsafe { self.array.get_unchecked(index) }
    }

    /// Updates the value of the connection matrix
    ///
    /// Copies the data array if needed and updates it in place
    /// Current implementation is not super for Rust because it
    /// violates borrowing rules, but
    /// 1. this object does not expose any references outside
    /// 2. usage of data still follows the pattern 1-mut xor many-read
    pub fn update(&mut self, left: u16, right: u16, value: i16) {
        let index = self.index(left, right);
        //need to check whether we own the connection matrix or not
        let data = if self.storage.is_some() {
            self.storage.as_mut().unwrap()
        } else {
            // copy borrowed data to the vector and replace slice
            let vec = Vec::from(self.array);
            let data: &'a [i16] = unsafe { std::mem::transmute(vec.as_slice()) };
            self.array = data;
            self.storage = Some(vec);
            self.storage.as_mut().unwrap()
        };
        data[index] = value;
        debug_assert_eq!(self.cost(left, right), value);
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
