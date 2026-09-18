/*
 * Copyright (c) 2021-2026 Works Applications Co., Ltd.
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

use crate::dic::word_id::EntryId;
use crate::util::cow_array::CowArray;

/// A word paratemter, used in the analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct WordParameter(u64);

impl WordParameter {
    /// The left connection cost id.
    #[inline]
    pub fn left_id(&self) -> i16 {
        (self.0 & 0xffff) as i16
    }

    /// The right connection cost id.
    #[inline]
    pub fn right_id(&self) -> i16 {
        ((self.0 >> 16) & 0xffff) as i16
    }

    /// The cost of the word.
    #[inline]
    pub fn cost(&self) -> i16 {
        ((self.0 >> 32) & 0xffff) as i16
    }

    /// Return a new parameters with the given cost.
    #[inline]
    pub fn with_cost(&self, cost: i16) -> WordParameter {
        WordParameter((cost as u64) << 32 | (self.0 & 0xffffffff))
    }
}

pub struct WordParams<'a> {
    // word parameters are placed at the beginning of eash word info and word infos are aligned per 8 bytes (see WordInfos::WORD_INFO_OFFSET_ALIGNMENT).
    // Thus handling this block as u64 array is safe.
    data: CowArray<'a, u64>,
}

impl<'a> WordParams<'a> {
    pub fn from_bytes(bytes: &'a [u8]) -> WordParams<'a> {
        Self {
            data: CowArray::from_bytes(bytes, 0, bytes.len() / std::mem::size_of::<u64>()),
        }
    }

    #[inline]
    pub fn get_params(&self, entry_id: EntryId) -> WordParameter {
        // offset in u64 array equals to the byte_offset >> 3, and byte_offset in V1 format is entry_id << 3.
        // Thus we can skip conversion
        let offset = entry_id.as_raw() as usize;
        WordParameter(self.data[offset])
    }

    #[inline]
    pub fn get_params_checked(&self, entry_id: EntryId) -> Option<WordParameter> {
        let offset = entry_id.as_raw() as usize;
        self.data.get(offset).copied().map(WordParameter)
    }

    #[inline]
    pub fn get_cost(&self, entry_id: EntryId) -> i16 {
        self.get_params(entry_id).cost()
    }

    pub fn set_cost(&mut self, entry_id: EntryId, cost: i16) {
        let offset = entry_id.as_raw() as usize;
        let params = self.get_params(entry_id);
        self.data.set(offset, params.with_cost(cost).0);
    }
}
