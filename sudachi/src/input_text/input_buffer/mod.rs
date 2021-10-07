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

pub use self::edit::EditInput;
use crate::dic::category_type::CategoryType;
use crate::dic::grammar::Grammar;
use std::ops::Range;

use crate::error::{SudachiError, SudachiResult};
use crate::input_text::InputTextIndex;

const MAX_LENGTH: usize = 32 * 1024;

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
    mod_category_continuity: Vec<usize>,
    /// This very temporaliy keeps the replace data.
    /// 'static lifetime is a lie, it is incorrect to use it outside `with_replacer` function
    /// or its callees.
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
        self.original.clear();
        self.modified.clear();
        self.m2o.clear();
        self.mod_chars.clear();
        self.mod_c2b.clear();
        self.mod_bow.clear();
        self.mod_category_continuity.clear();
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

    pub fn build(&mut self, _grammar: &Grammar) -> SudachiResult<()> {
        debug_assert_eq!(self.state, BufferState::RW);
        self.state = BufferState::RO;
        Ok(())
    }

    fn make_editor<'a>(&mut self) -> EditInput<'a> {
        // SAFETY: while it is possible to write into borrowed replaces
        // the buffer object itself will be accessible as RO
        let replaces: &'a mut Vec<edit::ReplaceOp<'a>> =
            unsafe { std::mem::transmute(&mut self.replaces) };
        return EditInput::new(replaces);
    }

    fn commit(&mut self) {
        if !self.replaces.is_empty() {
            self.mod_chars.clear()
        }
        edit::resolve_edits(
            &self.modified,
            &self.m2o,
            &mut self.modified_2,
            &mut self.m2o_2,
            &mut self.replaces,
        );
        std::mem::swap(&mut self.modified, &mut self.modified_2);
        std::mem::swap(&mut self.m2o, &mut self.m2o_2);
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
            Ok(_) => {
                self.commit();
                Ok(())
            }
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

    pub fn swap_original(&mut self, target: &mut String) {
        std::mem::swap(&mut self.original, target);
        self.original.clear();
        self.state = BufferState::Clean;
    }

    pub fn can_bow(&self, offset: usize) -> bool {
        debug_assert_eq!(self.state, BufferState::RO);
        let cpidx = self.mod_b2c[offset];
        self.mod_bow[cpidx]
    }

    pub fn get_word_candidate_length(&self, offset: usize) -> usize {
        debug_assert_eq!(self.state, BufferState::RO);
        todo!()
    }

    pub fn to_orig(&self, range: Range<usize>) -> Range<usize> {
        debug_assert_ne!(self.state, BufferState::Clean);
        self.m2o[range.start]..self.m2o[range.end]
    }
}

impl InputTextIndex for InputBuffer {
    fn cat_of_range(&self, range: Range<usize>) -> CategoryType {
        todo!()
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
        end - start - 1
    }

    fn cat_continuous_len(&self, offset: usize) -> usize {
        debug_assert_eq!(self.state, BufferState::RO);
        todo!()
    }

    fn byte_distance(&self, byte: usize, codepts: usize) -> usize {
        debug_assert_eq!(self.state, BufferState::RO);
        todo!()
    }

    fn orig_slice(&self, range: Range<usize>) -> &str {
        debug_assert_ne!(self.state, BufferState::Clean);
        &self.original[self.to_orig(range)]
    }

    fn curr_slice(&self, range: Range<usize>) -> &str {
        debug_assert_ne!(self.state, BufferState::Clean);
        &self.modified[range]
    }
}
