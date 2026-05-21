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

use std::iter::FusedIterator;

use crate::dic::read::varint::varint32;
use crate::dic::word_id::EntryId;

pub struct WordIdTable<'a> {
    bytes: &'a [u8],
}

impl<'a> WordIdTable<'a> {
    pub fn from_bytes(bytes: &'a [u8]) -> WordIdTable<'a> {
        WordIdTable { bytes }
    }

    pub fn new(bytes: &'a [u8], size: u32, offset: usize) -> WordIdTable<'a> {
        Self::from_bytes(&bytes[offset..offset + size as usize])
    }

    #[inline]
    pub fn entries(&self, index: usize) -> DeltaCompressedEntryIdIter<'a> {
        debug_assert!(index < self.bytes.len());
        DeltaCompressedEntryIdIter::new(&self.bytes[index..])
    }

    pub fn all_entries(&self) -> EntryIdIter<'a> {
        EntryIdIter {
            inner: DeltaCompressedEntryIdIter::new(self.bytes),
        }
    }
}

/// Iterator over word ids in a delta-compressed varint32 format.
pub struct DeltaCompressedEntryIdIter<'a> {
    pub(crate) rest: &'a [u8],
    remining: u32,
    sum: u32,
}

impl<'a> DeltaCompressedEntryIdIter<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        let (rest, remining) = varint32(bytes).expect("Failed to parse length in WordIdTable");

        DeltaCompressedEntryIdIter {
            rest,
            remining,
            sum: 0,
        }
    }
}

impl Iterator for DeltaCompressedEntryIdIter<'_> {
    type Item = EntryId;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.remining == 0 {
            return None;
        }

        let (rest, delta) = varint32(self.rest).expect("Failed to parse next word id delta");

        self.rest = rest;
        self.remining -= 1;
        self.sum += delta;
        Some(EntryId::new(self.sum))
    }
}

impl FusedIterator for DeltaCompressedEntryIdIter<'_> {}

/// Iterator over all word ids in the table.
pub struct EntryIdIter<'a> {
    inner: DeltaCompressedEntryIdIter<'a>,
}

impl Iterator for EntryIdIter<'_> {
    type Item = EntryId;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().or_else(|| {
            // If we reached the end of the inner iterator, move to the next list (if exists)
            if self.inner.rest.is_empty() {
                None
            } else {
                self.inner = DeltaCompressedEntryIdIter::new(self.inner.rest);
                self.next()
            }
        })
    }
}

impl FusedIterator for EntryIdIter<'_> {}
