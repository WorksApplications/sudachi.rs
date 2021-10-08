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

use std::ops::Range;

use crate::dic::category_type::CategoryType;
use crate::input_text::InputTextIndex;

/// Tokenization target text with original text and utility information
#[derive(Debug, Default)]
pub struct Utf8InputText<'a> {
    /// The original text
    pub original: &'a str,
    /// The text after preprocess. The tokenization works on this.
    pub modified: String,

    /// The mapping from modified byte_idx to original byte_idx.
    /// The mapped index locates at a valid char split point.
    /// Bytes in the middle of the char are mapped to the next char.
    offsets: Vec<usize>,
    /// The mapping from modified byte_idx to modified char_idx
    byte_indexes: Vec<usize>,

    /// Category types of each characters
    char_category_types: Vec<CategoryType>,
    /// Whether if the character can be a beginning of word
    can_bow_list: Vec<bool>,
    /// The byte_length to the next char where category type continuity ends
    char_category_continuities: Vec<usize>,
}

impl<'a> Utf8InputText<'a> {
    pub fn new(
        original: &'a str,
        modified: String,
        offsets: Vec<usize>,
        byte_indexes: Vec<usize>,
        char_category_types: Vec<CategoryType>,
        can_bow_list: Vec<bool>,
        char_category_continuities: Vec<usize>,
    ) -> Utf8InputText<'a> {
        Utf8InputText {
            original,
            modified,
            offsets,
            byte_indexes,
            char_category_types,
            can_bow_list,
            char_category_continuities,
        }
    }
}

impl Utf8InputText<'_> {
    /// Returns a substring of itself as a new Utf8InputText
    pub fn slice(&self, byte_start: usize, byte_end: usize) -> Utf8InputText {
        if byte_start == byte_end {
            return Utf8InputText::default();
        }

        let original = &self.original[self.offsets[byte_start]..self.offsets[byte_end]];
        let modified = self.modified[byte_start..byte_end].to_owned();

        let offset_base = self.offsets[byte_start];
        let offsets: Vec<_> = self.offsets[byte_start..byte_end + 1]
            .iter()
            .map(|v| v - offset_base)
            .collect();
        let byte_idx_base = self.byte_indexes[byte_start];
        let byte_indexes: Vec<_> = self.byte_indexes[byte_start..byte_end + 1]
            .iter()
            .map(|v| v - byte_idx_base)
            .collect();

        let char_category_types = self.char_category_types
            [self.byte_indexes[byte_start]..self.byte_indexes[byte_end]]
            .to_vec();
        let can_bow_list =
            self.can_bow_list[self.byte_indexes[byte_start]..self.byte_indexes[byte_end]].to_vec();

        let mut char_category_continuities =
            self.char_category_continuities[byte_start..byte_end].to_vec();
        if *char_category_continuities.last().unwrap() != 1 {
            for (i, idx) in (0..char_category_continuities.len()).rev().enumerate() {
                if char_category_continuities[idx] == 1 {
                    break;
                }
                char_category_continuities[idx] = i + 1;
            }
        }

        Utf8InputText {
            original,
            modified,
            offsets,
            byte_indexes,
            char_category_types,
            can_bow_list,
            char_category_continuities,
        }
    }

    /// Returns if the byte_idx can be a beginning of word
    pub fn can_bow(&self, byte_idx: usize) -> bool {
        // the byte must be a first byte of utf8 character
        (self.modified.as_bytes()[byte_idx] & 0xC0) != 0x80
            && self.can_bow_list[self.byte_indexes[byte_idx]]
    }

    /// Returns a substring of original text which corresponds to given byte range of modified text
    pub fn get_original_substring(&self, byte_range: Range<usize>) -> String {
        String::from(&self.original[self.offsets[byte_range.start]..self.offsets[byte_range.end]])
    }

    /// Returns a substring of modified text
    pub fn get_substring(&self, byte_range: Range<usize>) -> String {
        String::from(&self.modified[byte_range])
    }

    /// Returns a byte_length to the next can_bow point
    pub fn get_word_candidate_length(&self, byte_idx: usize) -> usize {
        // for SimpleOOV
        let byte_length = self.modified.len();
        for i in (byte_idx + 1)..byte_length {
            if self.can_bow(i) {
                return i - byte_idx;
            }
        }
        byte_length - byte_idx
    }

    /// Returns a category_types of the character at given byte_idx
    pub fn get_char_category_types(&self, byte_idx: usize) -> CategoryType {
        // for OOV and path_rewrite
        self.char_category_types[self.byte_indexes[byte_idx]]
    }

    /// Returns a common category_types of characters at given byte_range
    pub fn get_char_category_types_range(&self, byte_range: Range<usize>) -> CategoryType {
        if byte_range.is_empty() {
            return CategoryType::empty();
        }
        // for path_rewrite
        // this assumes b < e
        let b = self.byte_indexes[byte_range.start];
        let e = self.byte_indexes[byte_range.end];

        self.char_category_types[b..e]
            .iter()
            .fold(CategoryType::all(), |a, b| a & *b)
    }

    /// Returns byte length from byte_idx to index where category type continuity ends
    pub fn get_char_category_continuous_length(&self, byte_idx: usize) -> usize {
        // for MeCabOOV
        self.char_category_continuities[byte_idx]
    }

    /// Returns byte length from byte_idx to char code_point_offset after
    pub fn get_code_points_offset_length(
        &self,
        byte_idx: usize,
        code_point_offset: usize,
    ) -> usize {
        // for MeCabOOV and JoinKatakanaOOV
        let target = self.byte_indexes[byte_idx] + code_point_offset;
        for i in byte_idx..self.modified.len() {
            if self.byte_indexes[i] >= target {
                return i - byte_idx;
            }
        }
        self.modified.len() - byte_idx
    }

    /// Returns the number of characters in the given byte_range
    pub fn code_point_count(&self, byte_range: Range<usize>) -> usize {
        // for JoinKatakanaOOV
        self.byte_indexes[byte_range.end] - self.byte_indexes[byte_range.start]
    }

    /// Returns byte index of the next byte where original char changes
    pub fn get_next_in_original(&self, byte_idx: usize) -> usize {
        // for Sentencedetector
        let o = self.offsets[byte_idx + 1];
        for i in byte_idx..self.modified.len() {
            if self.offsets[i + 1] != o {
                return i;
            }
        }
        self.modified.len()
    }

    /// Returns corresponding byte index in the original test
    // this is for testing but exposed to use in plugin test
    pub fn get_original_index(&self, byte_idx: usize) -> usize {
        self.offsets[byte_idx]
    }
}

impl InputTextIndex for Utf8InputText<'_> {
    fn cat_of_range(&self, range: Range<usize>) -> CategoryType {
        self.get_char_category_types_range(range)
    }

    fn cat_at_byte(&self, offset: usize) -> CategoryType {
        self.get_char_category_types(offset)
    }

    fn num_codepts(&self, range: Range<usize>) -> usize {
        self.code_point_count(range)
    }

    fn cat_continuous_len(&self, offset: usize) -> usize {
        self.get_char_category_continuous_length(offset)
    }

    fn byte_distance(&self, byte: usize, codepts: usize) -> usize {
        self.get_code_points_offset_length(byte, codepts)
    }

    fn orig_slice(&self, range: Range<usize>) -> &str {
        &self.original[self.offsets[range.start]..self.offsets[range.end]]
    }

    fn curr_slice(&self, range: Range<usize>) -> &str {
        &self.modified[range]
    }

    fn to_orig(&self, range: Range<usize>) -> Range<usize> {
        self.get_original_index(range.start)..self.get_original_index(range.end)
    }
}
