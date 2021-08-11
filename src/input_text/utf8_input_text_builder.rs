use std::ops::Range;

use super::utf8_input_text::Utf8InputText;
use crate::dic::category_type::{CategoryType, CategoryTypes};
use crate::dic::grammar::Grammar;

pub struct Utf8InputTextBuilder<'a> {
    grammar: &'a Grammar<'a>,
    pub original: &'a str,
    pub modified: String,
    modified_to_original: Vec<usize>,
}

impl<'a> Utf8InputTextBuilder<'a> {
    pub fn new(original: &'a str, grammar: &'a Grammar) -> Utf8InputTextBuilder<'a> {
        let modified = String::from(original);

        let modified_to_original: Vec<usize> = modified
            .char_indices()
            .map(|(i, c)| vec![i; c.len_utf8()])
            .flatten()
            .chain([modified.len()])
            .collect();

        Utf8InputTextBuilder {
            grammar,
            original,
            modified,
            modified_to_original,
        }
    }

    fn char_range_to_byte_range(&self, char_range: Range<usize>) -> Range<usize> {
        let mut byte_start = 0;
        let mut byte_end = 0;
        for (char_idx, byte_idx) in self
            .modified
            .char_indices()
            .map(|v| v.0)
            .chain([self.modified.len()])
            .enumerate()
        {
            if char_idx == char_range.start {
                byte_start = byte_idx;
            }
            if char_idx == char_range.end {
                byte_end = byte_idx;
                break;
            }
        }
        byte_start..byte_end
    }

    pub fn replace(&mut self, char_range: Range<usize>, str_: &str) {
        let Range { start, end } = self.char_range_to_byte_range(char_range);

        // replace modified text
        self.modified.replace_range(start..end, str_);

        // update modified_to_original
        let length = str_.len();
        if length == 0 {
            self.modified_to_original.drain(start..end);
        } else {
            // the first char of replacing string will correspond with whole replaced string
            let modified_end = self.modified_to_original[end];
            self.modified_to_original
                .splice(start + 1..end, vec![modified_end; length - 1]);
        }
    }

    pub fn build(&self) -> Utf8InputText {
        let byte_indexes: Vec<usize> = self
            .modified
            .chars()
            .enumerate()
            .map(|(i, c)| vec![i; c.len_utf8()])
            .flatten()
            .chain([self.modified.chars().count()])
            .collect();
        let offsets = self.modified_to_original.clone();

        let char_category_types = self.build_char_category_types();
        let can_bow_list = self.build_can_bow_list(&char_category_types);
        let char_category_continuities =
            self.build_char_category_continuities(&char_category_types);

        Utf8InputText::new(
            self.original,
            &self.modified,
            offsets,
            byte_indexes,
            char_category_types,
            can_bow_list,
            char_category_continuities,
        )
    }

    fn build_char_category_types(&self) -> Vec<CategoryTypes> {
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

    fn get_char_category_continuous_length(
        char_category_types: &Vec<CategoryTypes>,
        c_offset: usize,
    ) -> usize {
        let mut continuous_cat = char_category_types[c_offset].clone();
        for length in 1..char_category_types.len() - c_offset {
            continuous_cat = continuous_cat
                .intersection(&char_category_types[c_offset + length])
                .map(|v| *v)
                .collect();
            if continuous_cat.is_empty() {
                return length;
            }
        }
        char_category_types.len() - c_offset
    }

    fn build_char_category_continuities(
        &self,
        char_category_types: &Vec<CategoryTypes>,
    ) -> Vec<usize> {
        if self.modified.is_empty() {
            return vec![];
        }

        let char_bound: Vec<_> = self
            .modified
            .char_indices()
            .map(|v| v.0)
            .chain([self.modified.len()])
            .collect();
        let mut continuities = vec![0; self.modified.len()];
        let mut ci = 0;
        while ci < char_category_types.len() {
            let clen =
                Utf8InputTextBuilder::get_char_category_continuous_length(&char_category_types, ci);
            let begin = char_bound[ci];
            let end = char_bound[ci + clen];
            for (i, v) in (0..end - begin).rev().enumerate() {
                continuities[begin + i] = v + 1;
            }
            ci += clen;
        }
        continuities
    }
}
