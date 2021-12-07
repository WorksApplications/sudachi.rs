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

pub use self::edit::InputEditor;
use crate::dic::category_type::CategoryType;
use crate::dic::grammar::Grammar;
use std::ops::Range;

use crate::error::{SudachiError, SudachiResult};
use crate::input_text::InputTextIndex;

/// limit on the maximum length of the input types, in bytes, 3/4 of u16::MAX
const MAX_LENGTH: usize = u16::MAX as usize / 4 * 3;

/// if the limit of the rewritten sentence is more than this number, then all bets are off
const REALLY_MAX_LENGTH: usize = u16::MAX as usize;

#[derive(Eq, PartialEq, Debug, Clone)]
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

/// InputBuffer - prepares the input data for the analysis
///
/// By saying char we actually mean Unicode codepoint here.
/// In the context of this struct these terms are synonyms.
#[derive(Default, Clone)]
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
    /// Buffer for normalization.
    /// After building it is used as byte-to-char mapping for original data.
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
    /// Creates new InputBuffer
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

    /// Moves InputBuffer into RW state, making it possible to perform edits on it
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

    /// Finalizes InputBuffer state, making it RO
    pub fn build(&mut self, grammar: &Grammar) -> SudachiResult<()> {
        debug_assert_eq!(self.state, BufferState::RW);
        self.state = BufferState::RO;
        self.mod_chars.clear();
        let cats = &grammar.character_category;
        let mut last_offset = 0;
        let mut last_chidx = 0;

        // Special cases for BOW logic
        let non_starting = CategoryType::ALPHA | CategoryType::GREEK | CategoryType::CYRILLIC;
        let mut prev_cat = CategoryType::empty();
        self.mod_bow.resize(self.modified.len(), false);
        let mut next_bow = true;

        for (chidx, (bidx, ch)) in self.modified.char_indices().enumerate() {
            self.mod_chars.push(ch);
            let cat = cats.get_category_types(ch);
            self.mod_cat.push(cat);
            self.mod_c2b.push(bidx);
            self.mod_b2c
                .extend(std::iter::repeat(last_chidx).take(bidx - last_offset));
            last_offset = bidx;
            last_chidx = chidx;

            let can_bow = if !next_bow {
                // this char was forbidden by the previous one
                next_bow = true;
                false
            } else if cat.intersects(CategoryType::NOOOVBOW2) {
                // this rule is stronger than the next one and must come before
                // this and next are forbidden
                next_bow = false;
                false
            } else if cat.intersects(CategoryType::NOOOVBOW) {
                // this char is forbidden
                false
            } else if cat.intersects(non_starting) {
                // the previous char is compatible
                !cat.intersects(prev_cat)
            } else {
                true
            };

            self.mod_bow[bidx] = can_bow;
            prev_cat = cat;
        }
        // trailing indices for the last codepoint
        self.mod_b2c
            .extend(std::iter::repeat(last_chidx).take(self.modified.len() - last_offset));
        // sentinel values for range translations
        self.mod_c2b.push(self.mod_b2c.len());
        self.mod_b2c.push(last_chidx + 1);

        self.fill_cat_continuity();
        self.fill_orig_b2c();

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

    fn fill_orig_b2c(&mut self) {
        self.m2o_2.clear();
        self.m2o_2.resize(self.original.len() + 1, usize::MAX);
        let mut max = 0;
        for (ch_idx, (b_idx, _)) in self.original.char_indices().enumerate() {
            self.m2o_2[b_idx] = ch_idx;
            max = ch_idx
        }
        self.m2o_2[self.original.len()] = max + 1;
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

    fn make_editor<'a>(&mut self) -> InputEditor<'a> {
        // SAFETY: while it is possible to write into borrowed replaces
        // the buffer object itself will be accessible as RO
        let replaces: &'a mut Vec<edit::ReplaceOp<'a>> =
            unsafe { std::mem::transmute(&mut self.replaces) };
        return InputEditor::new(replaces);
    }

    /// Execute a function which can modify the contents of the current buffer
    ///
    /// Edit can borrow &str from the context with the borrow checker working correctly     
    pub fn with_editor<'a, F>(&mut self, func: F) -> SudachiResult<()>
    where
        F: FnOnce(&InputBuffer, InputEditor<'a>) -> SudachiResult<InputEditor<'a>>,
        F: 'a,
    {
        debug_assert_eq!(self.state, BufferState::RW);
        // InputBufferReplacer should have 'a lifetime parameter for API safety
        // It is impossible to create it outside of this function
        // And the API forces user to return it by value
        let editor: InputEditor<'a> = self.make_editor();
        match func(self, editor) {
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
    /// Borrow original data
    pub fn original(&self) -> &str {
        debug_assert_ne!(self.state, BufferState::Clean);
        &self.original
    }

    /// Borrow modified data
    pub fn current(&self) -> &str {
        debug_assert_ne!(self.state, BufferState::Clean);
        &self.modified
    }

    /// Borrow array of current characters
    pub fn current_chars(&self) -> &[char] {
        debug_assert_ne!(self.state, BufferState::Clean);
        debug_assert_eq!(self.modified.is_empty(), self.mod_chars.is_empty());
        &self.mod_chars
    }

    /// Returns byte offsets of current chars
    pub fn curr_byte_offsets(&self) -> &[usize] {
        debug_assert_eq!(self.state, BufferState::RO);
        let len = self.mod_c2b.len();
        &self.mod_c2b[0..len - 1]
    }

    /// Get index of the current byte in original sentence
    /// Bytes not on character boundaries are not supported
    pub fn get_original_index(&self, index: usize) -> usize {
        debug_assert!(self.modified.is_char_boundary(index));
        self.m2o[index]
    }

    /// Mod Char Idx -> Orig Byte Idx
    pub fn to_orig_byte_idx(&self, index: usize) -> usize {
        debug_assert_ne!(self.state, BufferState::Clean);
        let byte_idx = self.mod_c2b[index];
        self.m2o[byte_idx]
    }

    /// Mod Char Idx -> Orig Char Idx
    pub fn to_orig_char_idx(&self, index: usize) -> usize {
        let b_idx = self.to_orig_byte_idx(index);
        let res = self.m2o_2[b_idx];
        debug_assert_ne!(res, usize::MAX);
        res
    }

    /// Mod Char Idx -> Mod Byte Idx
    pub fn to_curr_byte_idx(&self, index: usize) -> usize {
        debug_assert_eq!(self.state, BufferState::RO);
        self.mod_c2b[index]
    }

    /// Input: Mod Char Idx
    pub fn curr_slice_c(&self, data: Range<usize>) -> &str {
        debug_assert_eq!(self.state, BufferState::RO);
        let start = self.mod_c2b[data.start];
        let end = self.mod_c2b[data.end];
        &self.modified[start..end]
    }

    /// Input: Mod Char Idx
    pub fn orig_slice_c(&self, data: Range<usize>) -> &str {
        debug_assert_eq!(self.state, BufferState::RO);
        let start = self.to_orig_byte_idx(data.start);
        let end = self.to_orig_byte_idx(data.end);
        &self.original[start..end]
    }

    pub fn ch_idx(&self, idx: usize) -> usize {
        debug_assert_eq!(self.state, BufferState::RO);
        self.mod_b2c[idx]
    }

    /// Swaps original data with the passed location
    pub fn swap_original(&mut self, target: &mut String) {
        std::mem::swap(&mut self.original, target);
        self.state = BufferState::Clean;
    }

    /// Return original data as owned, consuming itself    
    pub fn into_original(self) -> String {
        self.original
    }

    /// Whether the byte can start a new word.
    /// Supports bytes not on character boundaries.
    #[inline]
    pub fn can_bow(&self, offset: usize) -> bool {
        debug_assert_eq!(self.state, BufferState::RO);
        self.mod_bow[offset]
    }

    /// Returns char length to the next can_bow point
    ///
    /// Used by SimpleOOV plugin
    pub fn get_word_candidate_length(&self, char_idx: usize) -> usize {
        debug_assert_eq!(self.state, BufferState::RO);
        let char_len = self.mod_chars.len();

        for i in (char_idx + 1)..char_len {
            let byte_idx = self.mod_c2b[i];
            if self.can_bow(byte_idx) {
                return i - char_idx;
            }
        }
        char_len - char_idx
    }
}

impl InputTextIndex for InputBuffer {
    #[inline]
    fn cat_of_range(&self, range: Range<usize>) -> CategoryType {
        debug_assert_eq!(self.state, BufferState::RO);
        if range.is_empty() {
            return CategoryType::empty();
        }

        self.mod_cat[range]
            .iter()
            .fold(CategoryType::all(), |a, b| a & *b)
    }

    #[inline]
    fn cat_at_char(&self, offset: usize) -> CategoryType {
        debug_assert_eq!(self.state, BufferState::RO);
        self.mod_cat[offset]
    }

    #[inline]
    fn cat_continuous_len(&self, offset: usize) -> usize {
        debug_assert_eq!(self.state, BufferState::RO);
        self.mod_cat_continuity[offset]
    }

    fn char_distance(&self, cpt: usize, offset: usize) -> usize {
        debug_assert_eq!(self.state, BufferState::RO);
        let end = (cpt + offset).min(self.mod_chars.len());
        end - cpt
    }

    #[inline]
    fn orig_slice(&self, range: Range<usize>) -> &str {
        debug_assert_ne!(self.state, BufferState::Clean);
        debug_assert!(
            self.modified.is_char_boundary(range.start),
            "start is off char boundary"
        );
        debug_assert!(
            self.modified.is_char_boundary(range.end),
            "end is off char boundary"
        );
        &self.original[self.to_orig(range)]
    }

    #[inline]
    fn curr_slice(&self, range: Range<usize>) -> &str {
        debug_assert_ne!(self.state, BufferState::Clean);
        &self.modified[range]
    }

    #[inline]
    fn to_orig(&self, range: Range<usize>) -> Range<usize> {
        debug_assert_ne!(self.state, BufferState::Clean);
        self.m2o[range.start]..self.m2o[range.end]
    }
}
