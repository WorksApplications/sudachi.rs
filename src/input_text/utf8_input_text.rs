use std::ops::Range;

use crate::dic::category_type::CategoryTypes;
use crate::prelude::*;

/// Tokenization target text with original text and utility information
#[derive(Debug)]
pub struct Utf8InputText<'a> {
    /// The original text
    pub original: &'a str,
    /// The text after preprocess. The tokenization works on this.
    pub modified: &'a str,

    /// mapping from modified byte_idx to original char_idx
    offsets: Vec<usize>,
    /// mapping from modified byte_idx to char_idx
    byte_indexes: Vec<usize>,

    /// category types of each characters
    char_category_types: Vec<CategoryTypes>,
    /// whether if the character can be a beginning of word
    can_bow_list: Vec<bool>,
    // byte_length to where category type continuity ends
    char_category_continuities: Vec<usize>,
}

impl<'a> Utf8InputText<'a> {
    pub fn new(
        original: &'a str,
        modified: &'a str,
        offsets: Vec<usize>,
        byte_indexes: Vec<usize>,
        char_category_types: Vec<CategoryTypes>,
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
    /// Returns if the byte_idx can be a beginning of word
    pub fn can_bow(&self, byte_idx: usize) -> bool {
        // the byte must be a first byte of utf8 character
        (self.modified.as_bytes()[byte_idx] & 0xC0) != 0x80
            && self.can_bow_list[self.byte_indexes[byte_idx]]
    }

    /// Returns a substring of original text which corresponds to given byte range of modified text
    pub fn get_original_substring(&self, range: Range<usize>) -> String {
        String::from(&self.original[self.offsets[range.start]..self.offsets[range.end]])
    }

    /// Returns a substring of modified text
    pub fn get_substring(&self, start: usize, end: usize) -> SudachiResult<String> {
        if end < start || self.modified.len() < end {
            return Err(SudachiError::InvalidRange(start, end));
        }
        Ok(String::from(&self.modified[start..end]))
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
    pub fn get_char_category_types(&self, byte_idx: usize) -> CategoryTypes {
        // for OOV and path_rewrite
        self.char_category_types[self.byte_indexes[byte_idx]].clone()
    }

    /// Returns a common category_types of characters at given byte_range
    pub fn get_char_category_types_range(&self, begin: usize, end: usize) -> CategoryTypes {
        // for path_rewrite
        let b = self.byte_indexes[begin];
        let e = self.byte_indexes[end];

        self.char_category_types[b..e]
            .iter()
            .map(|v| v.clone())
            .reduce(|a, b| a.intersection(&b).map(|v| *v).collect::<CategoryTypes>())
            .unwrap()
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
    pub fn code_point_count(&self, begin: usize, end: usize) -> usize {
        // for JoinKatakanaOOV
        self.byte_indexes[end] - self.byte_indexes[begin]
    }

    /// Returns corresponding byte index in the original test
    // this is for testing but exposed to use in plugin test
    pub fn get_original_index(&self, byte_idx: usize) -> usize {
        self.offsets[byte_idx]
    }
}
