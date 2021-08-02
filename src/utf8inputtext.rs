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

        let char_category_types = self.build_char_category_types();
        let can_bow_list = self.build_can_bow_list(&char_category_types);
        let char_category_continuities =
            self.build_char_category_continuities(&char_category_types);

        Utf8InputText {
            original: self.original,
            modified: &self.modified,
            offsets,
            byte_indexes,
            char_category_types,
            can_bow_list,
            char_category_continuities,
        }
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

#[derive(Debug)]
pub struct Utf8InputText<'a> {
    pub original: &'a str,
    pub modified: &'a str,

    // byte_idx to char_idx
    // todo?: rename?
    offsets: Vec<usize>,
    byte_indexes: Vec<usize>,

    // per char
    char_category_types: Vec<CategoryTypes>,
    can_bow_list: Vec<bool>,

    // per byte
    char_category_continuities: Vec<usize>,
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
        // for SimpleOOV
        let byte_length = self.modified.len();
        for i in (byte_idx + 1)..byte_length {
            if self.can_bow(i) {
                return i - byte_idx;
            }
        }
        byte_length - byte_idx
    }

    pub fn get_char_category_types(&self, byte_idx: usize) -> CategoryTypes {
        // for OOV
        self.char_category_types[self.byte_indexes[byte_idx]].clone()
    }

    pub fn get_char_category_types_range(&self, begin: usize, end: usize) -> CategoryTypes {
        // for path_rewrite
        let b = self.byte_indexes[begin];
        let e = self.byte_indexes[end];

        self.char_category_types[b..e]
            .iter()
            .fold(CategoryTypes::new(), |acc, set| {
                acc.intersection(&set).map(|v| *v).collect()
            })
    }

    pub fn get_char_category_continuous_length(&self, byte_idx: usize) -> usize {
        // for MeCabOOV
        // returns byte length from byte_idx to index where category continuity ends
        self.char_category_continuities[byte_idx]
    }

    pub fn get_code_points_offset_length(
        &self,
        byte_idx: usize,
        code_point_offset: usize,
    ) -> usize {
        // for MeCabOOV
        // return byte length from byte_idx to char code_point_offset after
        let target = self.byte_indexes[byte_idx] + code_point_offset;
        for i in byte_idx..self.modified.len() {
            if self.byte_indexes[i] >= target {
                return i - byte_idx;
            }
        }
        self.modified.len() - byte_idx
    }
}
