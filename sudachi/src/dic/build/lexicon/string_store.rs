/*
 *  Copyright (c) 2026 Works Applications Co., Ltd.
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

use std::cmp::Ordering;
use std::collections::HashMap;
use std::io::Write;

use crate::dic::build::error::{BuildFailure, DicBuildError};
use crate::dic::lexicon::strings::StringPointer;
use crate::error::SudachiResult;

use super::ResolvedLexiconEntry;

/// Compacted string storage for lexicon entries.
///
/// This mirrors Java `StringStorage`:
/// it reuses substring pointers when possible and delegates physical placement to `StringLayout`.
pub struct StringStore {
    strings: HashMap<String, Option<Item>>,
    candidates: HashMap<String, Item>,
    layout: StringLayout,
}

impl StringStore {
    /// Collect all required strings from entries and build compacted layout.
    pub fn from_entries(entries: &[ResolvedLexiconEntry]) -> SudachiResult<Self> {
        let mut st = Self {
            strings: HashMap::new(),
            candidates: HashMap::new(),
            layout: StringLayout::new(),
        };
        for e in entries {
            st.add(e.headword());
            st.add(e.reading());
        }
        st.compile()?;
        Ok(st)
    }

    /// Resolve an original string into the encoded `StringPointer`.
    pub fn resolve(&self, s: &str) -> StringPointer {
        self.strings
            .get(s)
            .and_then(|item| item.as_ref())
            .unwrap_or_else(|| panic!("string pointer missing for {}", s))
            .as_pointer()
    }

    /// Write compacted UTF-16 buffer.
    pub fn write<W: Write>(&self, w: &mut W) -> SudachiResult<usize> {
        self.layout.write(w)
    }

    /// Register a string before compaction.
    fn add(&mut self, data: &str) {
        self.strings.insert(data.to_owned(), None);
    }

    /// Run compaction:
    /// place longer strings first and create substring candidates from them.
    fn compile(&mut self) -> SudachiResult<()> {
        self.candidates.clear();
        self.candidates.insert(String::new(), Item::empty());

        let mut keys: Vec<String> = self.strings.keys().cloned().collect();
        keys.sort_by(|a, b| {
            utf16_len(b)
                .cmp(&utf16_len(a))
                .then_with(|| a.as_str().cmp(b.as_str()))
        });

        for k in keys {
            let item = self.process(&k)?;
            self.strings.insert(k, Some(item));
        }
        self.candidates.clear();
        Ok(())
    }

    /// Place one string or reuse an already discovered substring candidate.
    fn process(&mut self, s: &str) -> SudachiResult<Item> {
        if let Some(item) = self.candidates.get(s) {
            return Ok(item.clone());
        }

        let ptr = self.layout.add(s)?;
        let full = Item::full(s, ptr);
        self.candidates.insert(s.to_owned(), full.clone());

        let boundaries = char_boundaries(s);
        let num = boundaries.len() - 1;
        for i in 0..num {
            let start = boundaries[i];
            for end in boundaries[(i + 1)..].iter().copied() {
                let sub = &s[start.byte_idx..end.byte_idx];
                if !self.strings.contains_key(sub) || self.candidates.contains_key(sub) {
                    continue;
                }

                let sub_len = end.utf16_idx - start.utf16_idx;
                let sub_offset = ptr.offset + start.utf16_idx;
                if StringPointer::checked(sub_len, sub_offset).is_ok() {
                    let cand = Item::subseq(s, ptr, start.utf16_idx, end.utf16_idx);
                    self.candidates.insert(sub.to_owned(), cand);
                }
            }
        }

        Ok(full)
    }
}

/// Represents either a full string location or a substring range of a root string.
#[derive(Clone)]
struct Item {
    root_ptr: StringPointer,
    start: u32,
    end: u32,
}

impl Item {
    fn empty() -> Self {
        Self {
            root_ptr: StringPointer::unchecked(0, 0),
            start: 0,
            end: 0,
        }
    }

    fn full(data: &str, ptr: StringPointer) -> Self {
        Self {
            root_ptr: ptr,
            start: 0,
            end: utf16_len(data) as u32,
        }
    }

    fn subseq(_root: &str, root_ptr: StringPointer, start: u32, end: u32) -> Self {
        Self {
            root_ptr,
            start,
            end,
        }
    }

    /// Materialize the final pointer for this item (full or substring).
    fn as_pointer(&self) -> StringPointer {
        let len = self.end - self.start;
        let offset = self.root_ptr.offset + self.start;
        StringPointer::checked(len, offset).expect("invalid substring pointer")
    }
}

#[derive(Copy, Clone)]
struct Boundary {
    byte_idx: usize,
    utf16_idx: u32,
}

fn utf16_len(s: &str) -> usize {
    s.encode_utf16().count()
}

fn char_boundaries(s: &str) -> Vec<Boundary> {
    let mut result = Vec::with_capacity(s.chars().count() + 1);
    let mut utf16_idx: u32 = 0;
    for (byte_idx, ch) in s.char_indices() {
        result.push(Boundary {
            byte_idx,
            utf16_idx,
        });
        utf16_idx += ch.len_utf16() as u32;
    }
    result.push(Boundary {
        byte_idx: s.len(),
        utf16_idx,
    });
    result
}

/// UTF-16 aware placement engine for compacted strings.
///
/// It follows Java `StringLayout` strategy:
/// align by pointer constraints and reuse free slots created by alignment padding.
struct StringLayout {
    buffer: Vec<u16>,
    free: Vec<FreeSpace>,
    free_dirty: bool,
    pointer: usize,
    max_len: isize,
}

impl StringLayout {
    fn new() -> Self {
        Self {
            buffer: Vec::new(),
            free: Vec::new(),
            free_dirty: false,
            pointer: 0,
            max_len: -1,
        }
    }

    /// Allocate and place one string, returning its base pointer.
    fn add(&mut self, s: &str) -> SudachiResult<StringPointer> {
        let len = utf16_len(s);
        let alignment = required_alignment(len as u32);
        let offset = self.allocate(len, alignment);
        self.put(offset, s);
        StringPointer::checked(len as u32, offset as u32).map_err(|_| {
            DicBuildError {
                file: "<string store>".to_owned(),
                line: 0,
                cause: BuildFailure::InvalidStringPointer {
                    length: len,
                    offset,
                    alignment: alignment as usize,
                },
            }
            .into()
        })
    }

    /// Serialize only the used area (`pointer`) as little-endian UTF-16 bytes.
    fn write<W: Write>(&self, w: &mut W) -> SudachiResult<usize> {
        let mut bytes = Vec::with_capacity(self.pointer * 2);
        for u in self.buffer.iter().take(self.pointer) {
            bytes.extend_from_slice(&u.to_le_bytes());
        }
        w.write_all(&bytes)?;
        Ok(bytes.len())
    }

    fn put(&mut self, offset: usize, s: &str) {
        let data: Vec<u16> = s.encode_utf16().collect();
        let end = offset + data.len();
        if self.buffer.len() < end {
            self.buffer.resize(end, 0);
        }
        self.buffer[offset..end].copy_from_slice(&data);
    }

    /// Find a slot for `len` code units with alignment constraints.
    fn allocate(&mut self, len: usize, alignment: u32) -> usize {
        if len <= self.max_len.max(0) as usize {
            if self.free_dirty {
                self.free_dirty = false;
                self.free.sort();
            }

            let start_idx = self.free.partition_point(|f| f.length < len);
            for i in start_idx..self.free.len() {
                let fs = self.free[i];
                if fs.length < len {
                    continue;
                }
                let end = fs.start + fs.length;
                let Some(start) = self.allocate_in_block(len, alignment, fs.start, end) else {
                    continue;
                };
                let remaining = end - start - len;
                if remaining > 0 {
                    self.free[i] = FreeSpace {
                        start: start + len,
                        length: remaining,
                    };
                    self.free_dirty = true;
                } else {
                    self.free.remove(i);
                }
                self.recompute_max_len();
                return start;
            }
            self.max_len = std::cmp::max(0, self.max_len - 1);
        }

        let aligned = self
            .allocate_in_block(len, alignment, self.pointer, usize::MAX)
            .expect("allocation from tail must succeed");
        self.pointer = aligned + len;
        aligned
    }

    /// Try allocating inside a specific block; may emit a free slot for leading padding.
    fn allocate_in_block(
        &mut self,
        len: usize,
        alignment: u32,
        start: usize,
        end: usize,
    ) -> Option<usize> {
        let required_alignment = alignment.saturating_sub(1);
        let alignment_step = 1usize << required_alignment;
        let alignment_mask = alignment_step - 1;

        let mut aligned_start = start & !alignment_mask;
        let is_aligned = aligned_start == start;
        if !is_aligned {
            aligned_start += alignment_step;
        }

        let available = end.saturating_sub(aligned_start);
        if available < len {
            return None;
        }

        if !is_aligned {
            let padding = aligned_start - start;
            self.free.push(FreeSpace {
                start,
                length: padding,
            });
            self.free_dirty = true;
            self.max_len = std::cmp::max(self.max_len, available_max_len(padding) as isize);
        }
        Some(aligned_start)
    }

    fn recompute_max_len(&mut self) {
        self.max_len = self
            .free
            .iter()
            .map(|f| available_max_len(f.length) as isize)
            .max()
            .unwrap_or(-1);
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
struct FreeSpace {
    start: usize,
    length: usize,
}

impl Ord for FreeSpace {
    fn cmp(&self, other: &Self) -> Ordering {
        self.length
            .cmp(&other.length)
            .then_with(|| self.start.cmp(&other.start))
    }
}

impl PartialOrd for FreeSpace {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn required_alignment(length: u32) -> u32 {
    if length <= StringPointer::MAX_SIMPLE_LENGTH {
        0
    } else {
        let remaining = length - StringPointer::MAX_SIMPLE_LENGTH;
        32 - remaining.leading_zeros()
    }
}

fn available_max_len(length: usize) -> usize {
    let simple = (StringPointer::MAX_SIMPLE_LENGTH + 1) as usize;
    if length <= simple {
        return length;
    }
    let v = length - simple;
    let candidate = 1usize << ((usize::BITS - 1 - v.leading_zeros()) as usize);
    std::cmp::max(simple, candidate + simple)
}
