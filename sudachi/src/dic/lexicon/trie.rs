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
use std::iter::FusedIterator;

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct TrieEntry {
    /// Value of Trie, this is not the pointer to WordId, but the offset in WordId table
    pub value: u32,
    /// Offset of word end
    pub end: usize,
}

impl TrieEntry {
    #[inline]
    pub fn new(value: u32, offset: usize) -> TrieEntry {
        TrieEntry { value, end: offset }
    }
}

pub struct Trie<'a> {
    array: CowArray<'a, u32>,
}

pub struct TrieEntryIter<'a> {
    trie: &'a [u32],
    node_pos: usize,
    data: &'a [u8],
    offset: usize,
}

impl<'a> TrieEntryIter<'a> {
    #[inline(always)]
    fn get(&self, index: usize) -> u32 {
        debug_assert!(index < self.trie.len());
        // UB if out of bounds
        // Should we panic in release builds here instead?
        // Safe version is not optimized away
        *unsafe { self.trie.get_unchecked(index) }
    }
}

impl<'a> Iterator for TrieEntryIter<'a> {
    type Item = TrieEntry;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let mut node_pos = self.node_pos;
        let mut unit;

        for i in self.offset..self.data.len() {
            // Unwrap is safe: access is always in bounds
            // It is optimized away: https://rust.godbolt.org/z/va9K3az4n
            let k = self.data.get(i).unwrap();
            node_pos ^= *k as usize;
            unit = self.get(node_pos) as usize;
            if Trie::label(unit) != *k as usize {
                return None;
            }

            node_pos ^= Trie::offset(unit);
            if Trie::has_leaf(unit) {
                let r = TrieEntry::new(Trie::value(self.get(node_pos)), i + 1);
                self.offset = r.end;
                self.node_pos = node_pos;
                return Some(r);
            }
        }
        None
    }
}

impl FusedIterator for TrieEntryIter<'_> {}

impl<'a> Trie<'a> {
    pub fn new(data: &'a [u8], size: usize) -> Trie<'a> {
        Trie {
            array: CowArray::from_bytes(data, 0, size),
        }
    }

    pub fn new_owned(data: Vec<u32>) -> Trie<'a> {
        Trie {
            array: CowArray::from_owned(data),
        }
    }

    pub fn total_size(&self) -> usize {
        4 * self.array.len() as usize
    }

    #[inline]
    pub fn common_prefix_iterator<'b>(&'a self, input: &'b [u8], offset: usize) -> TrieEntryIter<'b>
    where
        'a: 'b,
    {
        let unit: usize = self.get(0) as usize;

        TrieEntryIter {
            node_pos: Trie::offset(unit),
            data: input,
            trie: &self.array,
            offset,
        }
    }

    #[inline(always)]
    fn get(&self, index: usize) -> u32 {
        debug_assert!(index < self.array.len());
        // UB if out of bounds
        // Should we panic in release builds here instead?
        // Safe version is not optimized away
        *unsafe { self.array.get_unchecked(index) }
    }

    #[inline(always)]
    fn has_leaf(unit: usize) -> bool {
        ((unit >> 8) & 1) == 1
    }

    #[inline(always)]
    fn value(unit: u32) -> u32 {
        unit & ((1 << 31) - 1)
    }

    #[inline(always)]
    fn label(unit: usize) -> usize {
        unit & ((1 << 31) | 0xFF)
    }

    #[inline(always)]
    fn offset(unit: usize) -> usize {
        (unit >> 10) << ((unit & (1 << 9)) >> 6)
    }
}
