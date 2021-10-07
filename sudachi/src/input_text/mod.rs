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

pub mod input_buffer;
pub mod utf8_input_text;
pub mod utf8_input_text_builder;

use crate::dic::category_type::CategoryType;
use std::ops::Range;
pub use utf8_input_text::Utf8InputText;
pub use utf8_input_text_builder::Utf8InputTextBuilder;

/// Provides fast indexed access into the input text
pub trait InputTextIndex {
    /// Common character category inside the range
    fn cat_of_range(&self, range: Range<usize>) -> CategoryType;

    /// Character category at byte offset
    fn cat_at_byte(&self, offset: usize) -> CategoryType;

    /// Number of codepoints in the range indexed by byte indices
    fn num_codepts(&self, range: Range<usize>) -> usize;

    /// Number of codepoints to the right of the offset with the same character category
    ///
    /// Java name: getCharCategoryContinuousLength
    fn cat_continuous_len(&self, offset: usize) -> usize;

    /// Distance in bytes between the char indexed by `byte`
    /// and the char, relative to it by `codepts`.
    fn byte_distance(&self, byte: usize, codepts: usize) -> usize;

    /// Returns substring of original text by indices from the current text
    fn orig_slice(&self, range: Range<usize>) -> &str;

    /// Returns substring of the current (modified) text by indices from the current text
    fn curr_slice(&self, range: Range<usize>) -> &str;
}

#[cfg(test)]
mod u8_text_tests;
