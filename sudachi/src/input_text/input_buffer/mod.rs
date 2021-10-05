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

use crate::error::SudachiResult;
use crate::input_text::PathRewriteAPI;

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
    original: String,
    modified: String,
    modified2: String,
    m2o_bytes: Vec<usize>,
    m2o_bytes2: Vec<usize>,
    mod_chars: Vec<char>,
    mod_c2b: Vec<usize>,
    mod_b2c: Vec<usize>,
    mod_bow: Vec<bool>,
    mod_category_continuity: Vec<usize>,
    replaces: Vec<edit::ReplaceOp<'static>>,
    state: BufferState,
}

impl InputBuffer {
    pub fn new() -> InputBuffer {
        InputBuffer::default()
    }

    pub fn reset(&mut self) -> &mut String {
        self.original.clear();
        self.modified.clear();
        self.m2o_bytes.clear();
        self.mod_chars.clear();
        self.mod_c2b.clear();
        self.mod_bow.clear();
        self.mod_category_continuity.clear();
        self.state = BufferState::Clean;
        &mut self.original
    }

    pub fn from<'a, T: AsRef<str>>(data: T) -> InputBuffer {
        let mut buf = Self::new();
        buf.reset().push_str(data.as_ref());
        buf.start_build();
        buf
    }

    pub fn start_build(&mut self) {
        debug_assert_eq!(self.state, BufferState::Clean);
        self.state = BufferState::RW;
        self.modified.push_str(&self.original);
        self.m2o_bytes.extend(0..self.modified.len());
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
            &self.m2o_bytes,
            &mut self.modified2,
            &mut self.m2o_bytes2,
            &mut self.replaces,
        );
        std::mem::swap(&mut self.modified, &mut self.modified2);
        std::mem::swap(&mut self.m2o_bytes, &mut self.m2o_bytes2);
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
        todo!()
    }

    pub fn get_word_candidate_length(&self, offset: usize) -> usize {
        todo!()
    }

    pub fn orig_slice(&self, range: Range<usize>) -> &str {
        todo!()
    }
}

impl PathRewriteAPI for InputBuffer {
    fn cat_of_range(&self, range: Range<usize>) -> CategoryType {
        todo!()
    }

    fn cat_at_byte(&self, offset: usize) -> CategoryType {
        todo!()
    }

    fn num_codepts(&self, range: Range<usize>) -> usize {
        let start = self.mod_b2c[range.start];
        let end = self.mod_b2c[range.end];
        end - start
    }
}
