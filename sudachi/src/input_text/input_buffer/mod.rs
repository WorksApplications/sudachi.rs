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

mod edit;
#[cfg(test)]
mod test_basic;
#[cfg(test)]
mod test_ported;

pub use self::edit::EditInput;
use crate::dic::category_type::CategoryType;
use crate::dic::grammar::Grammar;
use std::ops::Range;

use crate::error::{SudachiError, SudachiResult};
use crate::input_text::InputTextIndex;

/// limit on the maximum length of the input types, in bytes, 4/3 of u16::MAX
const MAX_LENGTH: usize = u16::MAX as usize / 4 * 3;

/// if the limit of the rewritten sentence is more than this number, then all bets are off
const REALLY_MAX_LENGTH: usize = u16::MAX as usize;

#[derive(Eq, PartialEq, Debug)]
enum BufferState {
    Clean,
    RW,
    RO,
}

impl Default for BufferState {
    fn default() -> Self {
        BufferState::Clean
    }
}

/// InputBuffer
///
/// By saying char we actually mean Unicode codepoint here.
/// In the context of this struct these terms are synonyms.
#[derive(Default)]
pub struct InputBuffer {
    /// Original input data, output is done on this
    original: String,
    /// Normalized input data, analysis is done on this. Byte-based indexing.
    modified: String,
    /// Buffer for normalization, reusing allocations
    modified_2: String,
    /// Byte mapping from normalized data to originals.
    /// Only values lying on codepoint boundaries are correct. Byte-based indexing.
    m2o: Vec<usize>,
    /// Buffer for normalization
    m2o_2: Vec<usize>,
    /// Characters of the modified string. Char-based indexing.
    mod_chars: Vec<char>,
    /// Char-to-byte mapping for the modified string. Char-based indexing.
    mod_c2b: Vec<usize>,
    /// Byte-to-char mapping for the modified string. Byte-based indexing.
    mod_b2c: Vec<usize>,
    /// Markers whether the byte can start new word or not
    mod_bow: Vec<bool>,
    /// Character categories. Char-based indexing.
    mod_cat: Vec<CategoryType>,
    /// Number of codepoints with the same category. Char-based indexing.
    mod_cat_continuity: Vec<usize>,
    /// This very temporarily keeps the replacement data.
    /// 'static lifetime is a lie and it is **incorrect** to use
    /// it outside `with_replacer` function or its callees.
    replaces: Vec<edit::ReplaceOp<'static>>,
    /// Current state of the buffer
    state: BufferState,
}

impl InputBuffer {
    pub fn new() -> InputBuffer {
        InputBuffer::default()
    }

    /// Resets the input buffer, so it could be used to process new input.
    /// New input should be written to the returned mutable reference.
    pub fn reset(&mut self) -> &mut String {
        // extended buffers can be ignored during cleaning,
        // they will be cleaned before usage automatically
        self.original.clear();
        self.modified.clear();
        self.m2o.clear();
        self.mod_chars.clear();
        self.mod_c2b.clear();
        self.mod_b2c.clear();
        self.mod_bow.clear();
        self.mod_cat.clear();
        self.mod_cat_continuity.clear();
        self.state = BufferState::Clean;
        &mut self.original
    }

    /// Creates input from the passed string. Should be used mostly for tests.
    ///
    /// Panics if the input string is too long.
    pub fn from<'a, T: AsRef<str>>(data: T) -> InputBuffer {
        let mut buf = Self::new();
        buf.reset().push_str(data.as_ref());
        buf.start_build().expect("");
        buf
    }

    pub fn start_build(&mut self) -> SudachiResult<()> {
        if self.original.len() > MAX_LENGTH {
            return Err(SudachiError::InputTooLong(self.original.len(), MAX_LENGTH));
        }
        debug_assert_eq!(self.state, BufferState::Clean);
        self.state = BufferState::RW;
        self.modified.push_str(&self.original);
        self.m2o.extend(0..self.modified.len() + 1);
        Ok(())
    }

    pub fn build(&mut self, grammar: &Grammar) -> SudachiResult<()> {
        debug_assert_eq!(self.state, BufferState::RW);
        self.state = BufferState::RO;
        self.mod_chars.clear();
        let cats = &grammar.character_category;
        let mut last_offset = 0;
        let mut last_chidx = 0;

        let non_starting = CategoryType::ALPHA | CategoryType::GREEK | CategoryType::CYRILLIC;
        let mut prev_cat = CategoryType::empty();
        self.mod_bow.resize(self.modified.len(), false);

        for (chidx, (bidx, ch)) in self.modified.char_indices().enumerate() {
            self.mod_chars.push(ch);
            let cat = cats.get_category_types(ch);
            self.mod_cat.push(cat);
            self.mod_c2b.push(bidx);
            self.mod_b2c
                .extend(std::iter::repeat(last_chidx).take(bidx - last_offset));
            last_offset = bidx;
            last_chidx = chidx;

            self.mod_bow[bidx] = if cat.intersects(non_starting) {
                !cat.intersects(prev_cat)
            } else {
                true
            };
            prev_cat = cat;
        }
        // trailing indices for the last codepoint
        self.mod_b2c
            .extend(std::iter::repeat(last_chidx).take(self.modified.len() - last_offset));
        // sentinel values for range translations
        self.mod_c2b.push(self.mod_b2c.len());
        self.mod_b2c.push(last_chidx + 1);

        self.fill_cat_continuity();

        Ok(())
    }

    fn fill_cat_continuity(&mut self) {
        if self.mod_chars.is_empty() {
            return;
        }
        // single pass algorithm
        // by default continuity is 1 codepoint
        // go from the back and set it prev + 1 when chars are compatible
        self.mod_cat_continuity.resize(self.mod_chars.len(), 1);
        let mut cat = *self.mod_cat.last().unwrap_or(&CategoryType::all());
        for i in (0..self.mod_cat.len() - 1).rev() {
            let cur = self.mod_cat[i];
            let common = cur & cat;
            if !common.is_empty() {
                self.mod_cat_continuity[i] = self.mod_cat_continuity[i + 1] + 1;
                cat = common;
            } else {
                cat = cur;
            }
        }
    }

    fn make_editor<'a>(&mut self) -> EditInput<'a> {
        // SAFETY: while it is possible to write into borrowed replaces
        // the buffer object itself will be accessible as RO
        let replaces: &'a mut Vec<edit::ReplaceOp<'a>> =
            unsafe { std::mem::transmute(&mut self.replaces) };
        return EditInput::new(replaces);
    }

    fn commit(&mut self) -> SudachiResult<()> {
        if self.replaces.is_empty() {
            return Ok(());
        }

        self.mod_chars.clear();
        self.modified_2.clear();
        self.m2o_2.clear();

        let sz = edit::resolve_edits(
            &self.modified,
            &self.m2o,
            &mut self.modified_2,
            &mut self.m2o_2,
            &mut self.replaces,
        );
        if sz > REALLY_MAX_LENGTH {
            // super improbable, but still
            return Err(SudachiError::InputTooLong(sz, REALLY_MAX_LENGTH));
        }
        std::mem::swap(&mut self.modified, &mut self.modified_2);
        std::mem::swap(&mut self.m2o, &mut self.m2o_2);
        Ok(())
    }

    fn rollback(&mut self) {
        self.replaces.clear()
    }

    /// Execute a function which can modify the contents of the current buffer
    ///
    /// Must perform
    pub fn with_replacer<'a, F>(&mut self, func: F) -> SudachiResult<()>
    where
        F: FnOnce(&InputBuffer, EditInput<'a>) -> SudachiResult<EditInput<'a>>,
        F: 'a,
    {
        debug_assert_eq!(self.state, BufferState::RW);
        // InputBufferReplacer should have 'a lifetime parameter for API safety
        // It is impossible to create it outside of this function
        // And the API forces user to return it by value
        let replacer: EditInput<'a> = self.make_editor();
        match func(self, replacer) {
            Ok(_) => self.commit(),
            Err(e) => {
                self.rollback();
                Err(e)
            }
        }
    }

    /// Recompute chars from modified string (useful if the processing will use chars)
    pub fn refresh_chars(&mut self) {
        debug_assert_eq!(self.state, BufferState::RW);
        if self.mod_chars.is_empty() {
            self.mod_chars.extend(self.modified.chars());
        }
    }
}

// RO Accessors
impl InputBuffer {
    pub fn original(&self) -> &str {
        debug_assert_ne!(self.state, BufferState::Clean);
        &self.original
    }

    pub fn current(&self) -> &str {
        debug_assert_ne!(self.state, BufferState::Clean);
        &self.modified
    }

    pub fn current_chars(&self) -> &[char] {
        debug_assert_ne!(self.state, BufferState::Clean);
        debug_assert_eq!(self.modified.is_empty(), self.mod_chars.is_empty());
        &self.mod_chars
    }

    pub fn get_original_index(&self, index: usize) -> usize {
        self.m2o[index]
    }

    pub fn swap_original(&mut self, target: &mut String) {
        std::mem::swap(&mut self.original, target);
        self.state = BufferState::Clean;
    }

    pub fn can_bow(&self, offset: usize) -> bool {
        debug_assert_eq!(self.state, BufferState::RO);
        self.mod_bow[offset]
    }

    /// Returns a byte_length to the next can_bow point
    ///
    /// Used by SimpleOOV plugin
    pub fn get_word_candidate_length(&self, byte_idx: usize) -> usize {
        debug_assert_eq!(self.state, BufferState::RO);
        let byte_length = self.modified.len();

        for i in (byte_idx + 1)..byte_length {
            if self.can_bow(i) {
                return i - byte_idx;
            }
        }
        byte_length - byte_idx
    }
}

impl InputTextIndex for InputBuffer {
    fn cat_of_range(&self, range: Range<usize>) -> CategoryType {
        debug_assert_eq!(self.state, BufferState::RO);
        let range_c = self.mod_b2c[range.start]..self.mod_b2c[range.end];
        self.mod_cat[range_c]
            .iter()
            .fold(CategoryType::all(), |a, b| a & *b)
    }

    fn cat_at_byte(&self, offset: usize) -> CategoryType {
        debug_assert_eq!(self.state, BufferState::RO);
        let cpidx = self.mod_b2c[offset];
        self.mod_cat[cpidx]
    }

    fn num_codepts(&self, range: Range<usize>) -> usize {
        debug_assert_eq!(self.state, BufferState::RO);
        let start = self.mod_b2c[range.start];
        let end = self.mod_b2c[range.end];
        end - start
    }

    fn cat_continuous_len(&self, offset: usize) -> usize {
        debug_assert_eq!(self.state, BufferState::RO);
        let start_c = self.mod_b2c[offset];
        let length = self.mod_cat_continuity[start_c];
        let end_c = start_c + length;
        self.mod_c2b[end_c] - offset
    }

    fn byte_distance(&self, byte: usize, codepts: usize) -> usize {
        debug_assert_eq!(self.state, BufferState::RO);
        let start_c = self.mod_b2c[byte];
        let tgt_c = (start_c + codepts).min(self.mod_chars.len());
        self.mod_c2b[tgt_c] - byte
    }

    fn orig_slice(&self, range: Range<usize>) -> &str {
        debug_assert_ne!(self.state, BufferState::Clean);
        debug_assert!(self.modified.is_char_boundary(range.start));
        debug_assert!(self.modified.is_char_boundary(range.end));
        &self.original[self.to_orig(range)]
    }

    fn curr_slice(&self, range: Range<usize>) -> &str {
        debug_assert_ne!(self.state, BufferState::Clean);
        &self.modified[range]
    }

    fn to_orig(&self, range: Range<usize>) -> Range<usize> {
        debug_assert_ne!(self.state, BufferState::Clean);
        self.m2o[range.start]..self.m2o[range.end]
    }
}
