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
use std::ptr::NonNull;

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

    #[inline]
    pub fn entries(&self, index: usize) -> WordIdIter {
        debug_assert!(index < self.bytes.len());
        let ptr = unsafe { self.bytes.as_ptr().offset((index + self.offset) as isize) };
        let cnt = unsafe { ptr.read() } as usize;
        let data_ptr = unsafe { ptr.offset(1) } as *const u32;
        debug_assert!(index + cnt * std::mem::size_of::<u32>() + 1 <= self.bytes.len());
        WordIdIter {
            data: unsafe { NonNull::new_unchecked(data_ptr as _) },
            remaining: cnt,
        }
    }
}

pub struct WordIdIter {
    /// This pointer is unaligned and must be read from using unaligned reads.
    /// Using NonNull makes Option<Self> be the same as the struct itself.
    data: NonNull<u32>,
    /// number of remaining elements
    remaining: usize,
}

impl Iterator for WordIdIter {
    type Item = u32;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        let ptr = self.data.as_ptr();

        let val = unsafe { ptr.read_unaligned() };
        self.data = unsafe { NonNull::new_unchecked(ptr.offset(1)) };
        self.remaining -= 1;
        Some(val)
    }
}

impl FusedIterator for WordIdIter {}
