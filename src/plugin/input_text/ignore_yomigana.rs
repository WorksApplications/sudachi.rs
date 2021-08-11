use std::collections::HashSet;

use crate::dic::category_type::CategoryType;
use crate::dic::character_category::CharacterCategory;
use crate::dic::grammar::Grammar;
use crate::input_text::utf8_input_text_builder::Utf8InputTextBuilder;
use crate::plugin::input_text::InputTextPlugin;
use crate::prelude::*;

pub struct IgnoreYomiganaPlugin<'a> {
    character_category: CharacterCategory,
    left_bracket_set: HashSet<&'a char>,
    right_bracket_set: HashSet<&'a char>,
    max_yomigana_length: usize,
}

impl<'a> IgnoreYomiganaPlugin<'a> {
    pub fn new(grammar: &Grammar) -> SudachiResult<IgnoreYomiganaPlugin<'a>> {
        // todo: load from config
        let left_bracket_set = ['(', '（'].iter().collect();
        let right_bracket_set = [')', '）'].iter().collect();
        let max_yomigana_length = 4;

        Ok(IgnoreYomiganaPlugin {
            // todo: better way to use char_category map?
            character_category: grammar.character_category.clone(),
            left_bracket_set,
            right_bracket_set,
            max_yomigana_length,
        })
    }

    fn has_category_type(&self, c: char, t: &CategoryType) -> bool {
        self.character_category.get_category_types(c).contains(t)
    }
    fn is_kanji(&self, c: char) -> bool {
        self.has_category_type(c, &CategoryType::KANJI)
    }
    fn is_hiragana(&self, c: char) -> bool {
        self.has_category_type(c, &CategoryType::HIRAGANA)
    }
    fn is_katakana(&self, c: char) -> bool {
        self.has_category_type(c, &CategoryType::KATAKANA)
    }
}

impl<'a> InputTextPlugin for IgnoreYomiganaPlugin<'a> {
    fn rewrite(&self, builder: &mut Utf8InputTextBuilder) {
        let chars: Vec<_> = builder.modified.chars().collect();
        let mut start_bracket_point = None;
        let mut offset = 0;
        let mut has_yomigana = false;
        for i in 1..chars.len() {
            if self.is_kanji(chars[i - 1]) && self.left_bracket_set.contains(&chars[i]) {
                start_bracket_point = Some(i);
                continue;
            }
            if has_yomigana && self.right_bracket_set.contains(&chars[i]) {
                let start = start_bracket_point.unwrap();
                let replace: String = chars[start - 1..start].iter().collect();
                builder.replace(start - 1 - offset..i + 1 - offset, &replace);
                offset += i - start + 1;
                start_bracket_point = None;
                has_yomigana = false;
                continue;
            }
            if let Some(start) = start_bracket_point {
                if (self.is_hiragana(chars[i]) || self.is_katakana(chars[i]))
                    && i - start <= self.max_yomigana_length
                {
                    has_yomigana = true;
                } else {
                    start_bracket_point = None;
                    has_yomigana = false;
                }
                continue;
            }
        }
    }
}
