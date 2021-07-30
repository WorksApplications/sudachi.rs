use crate::dic::category_type::{CategoryType, CategoryTypes};
use crate::dic::grammar::Grammar;
use crate::prelude::*;

pub struct Utf8InputTextBuilder<'a> {
    grammar: &'a Grammar<'a>,
    pub original: &'a str,
    pub modified: String,
    modified_to_original: Vec<usize>,
}

impl<'a> Utf8InputTextBuilder<'a> {
    pub fn new(original: &'a str, grammar: &'a Grammar) -> Utf8InputTextBuilder<'a> {
        let modified = String::from(original);

        let char_length = modified.chars().count();
        let modified_to_original: Vec<usize> = (0..char_length + 1).collect();

        Utf8InputTextBuilder {
            grammar,
            original,
            modified,
            modified_to_original,
        }
    }

    pub fn build(&self) -> Utf8InputText {
        let byte_length = self.modified.len();
        let mut byte_indexes = vec![0; byte_length + 1];
        let mut offsets = vec![0; byte_length + 1];
        let mut left = 0;
        for (char_idx, right) in self
            .modified
            .char_indices()
            .map(|v| v.0)
            .chain([byte_length + 1])
            .skip(1)
            .enumerate()
        {
            for i in left..right {
                byte_indexes[i] = char_idx;
                offsets[i] = self.modified_to_original[char_idx];
            }
            left = right;
        }
        byte_indexes[byte_length] = self.modified_to_original.len();
        offsets[byte_length] = *self.modified_to_original.last().unwrap();

        let char_category_types = self.get_char_category_types();
        let can_bow_list = self.build_can_bow_list(&char_category_types);

        Utf8InputText {
            original: self.original,
            modified: &self.modified,
            offsets,
            byte_indexes,
            can_bow_list,
        }
    }

    fn get_char_category_types(&self) -> Vec<CategoryTypes> {
        self.modified
            .chars()
            .map(|c| self.grammar.character_category.get_category_types(c))
            .collect()
    }

    fn build_can_bow_list(&self, char_category_types: &Vec<CategoryTypes>) -> Vec<bool> {
        if self.modified.is_empty() {
            return vec![];
        }

        let mut can_bow_list = vec![true; char_category_types.len()];
        for (i, cat) in char_category_types.iter().enumerate() {
            if i == 0 {
                continue;
            }

            // in rust, char corresponds to unicode scalar value
            // and we do not need to check surrogate

            if cat.contains(&CategoryType::ALPHA)
                || cat.contains(&CategoryType::GREEK)
                || cat.contains(&CategoryType::CYRILLIC)
            {
                // can bow if previous charactar does not have same category type
                can_bow_list[i] = cat.intersection(&char_category_types[i - 1]).count() == 0;
            }
        }

        can_bow_list
    }
}

#[derive(Debug)]
pub struct Utf8InputText<'a> {
    pub original: &'a str,
    pub modified: &'a str,
    offsets: Vec<usize>,
    // todo?: rename to byte2char_index
    byte_indexes: Vec<usize>,

    can_bow_list: Vec<bool>,
}

impl Utf8InputText<'_> {
    pub fn can_bow(&self, byte_idx: usize) -> bool {
        (self.modified.as_bytes()[byte_idx] & 0xC0) != 0x80
            && self.can_bow_list[self.byte_indexes[byte_idx]]
    }

    pub fn get_substring(&self, start: usize, end: usize) -> SudachiResult<String> {
        if end < start || self.modified.len() < end {
            return Err(SudachiError::InvalidRange(start, end));
        }
        Ok(String::from(&self.modified[start..end]))
    }

    pub fn get_word_candidate_length(&self, byte_idx: usize) -> usize {
        let byte_length = self.modified.len();
        for i in (byte_idx + 1)..byte_length {
            if self.can_bow(i) {
                return i - byte_idx;
            }
        }
        byte_length - byte_idx
    }
}
