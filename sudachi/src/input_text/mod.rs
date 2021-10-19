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

mod buffer;

use crate::dic::category_type::CategoryType;
use std::ops::Range;

pub use self::buffer::InputBuffer;
pub use self::buffer::InputEditor;

/// Provides fast indexed access into the input text
pub trait InputTextIndex {
    /// Common character category inside the range. Indexed by chars.
    fn cat_of_range(&self, range: Range<usize>) -> CategoryType;

    /// Character category at char offset
    fn cat_at_char(&self, offset: usize) -> CategoryType;

    /// Number of chars to the right of the offset with the same character category
    ///
    /// Java name: getCharCategoryContinuousLength
    fn cat_continuous_len(&self, offset: usize) -> usize;

    /// Distance in chars between the char indexed by `index`
    /// and the char, relative to it by `offset`.
    /// Java name: getCodePointsOffsetLength
    fn char_distance(&self, index: usize, offset: usize) -> usize;

    /// Returns substring of original text by indices from the current text
    fn orig_slice(&self, range: Range<usize>) -> &str;

    /// Returns substring of the current (modified) text by indices from the current text
    fn curr_slice(&self, range: Range<usize>) -> &str;

    /// Translate range from current state to original. Byte-indexed.
    fn to_orig(&self, range: Range<usize>) -> Range<usize>;
}
