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

use std::intrinsics::offset;
use std::iter::FusedIterator;

/// Read UTF-16 code units from non-aligned storage
pub struct U16CodeUnits<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> U16CodeUnits<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        U16CodeUnits { data, offset: 0 }
    }
}

impl Iterator for U16CodeUnits<'_> {
    type Item = u16;

    fn next(&mut self) -> Option<Self::Item> {
        if self.data.len() <= self.offset {
            return None;
        }
        let p1 = self.data[self.offset];
        let p2 = self.data[self.offset + 1];
        self.offset += 2;
        Some(u16::from_le_bytes([p1, p2]))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let rem = self.data.len() - self.offset;
        (rem, Some(rem))
    }
}

impl FusedIterator for U16CodeUnits<'_> {}
