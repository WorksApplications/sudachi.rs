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

use regex::Regex;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashSet;
use std::fmt::Write;
use std::ops::Range;

use crate::config::Config;
use crate::dic::category_type::CategoryType;
use crate::dic::character_category::CharacterCategory;
use crate::dic::grammar::Grammar;
use crate::input_text::input_buffer::{EditInput, InputBuffer};
use crate::input_text::Utf8InputTextBuilder;
use crate::plugin::input_text::InputTextPlugin;
use crate::plugin::PluginError;
use crate::prelude::*;

#[cfg(test)]
mod tests;

/// Search katakana in the bracket after kanji character as Yomigana (読み仮名)
/// and removes it from tokenization target
#[derive(Default)]
pub struct IgnoreYomiganaPlugin {
    character_category: CharacterCategory,
    left_bracket_set: HashSet<char>,
    right_bracket_set: HashSet<char>,
    max_yomigana_length: usize,
    regex: Option<Regex>,
}

/// Struct corresponds with raw config json file.
#[allow(non_snake_case)]
#[derive(Deserialize)]
struct PluginSettings {
    leftBrackets: Vec<char>,
    rightBrackets: Vec<char>,
    maxYomiganaLength: usize,
}

impl IgnoreYomiganaPlugin {
    fn has_category_type(&self, c: char, t: CategoryType) -> bool {
        self.character_category.get_category_types(c).contains(t)
    }
    fn is_kanji(&self, c: char) -> bool {
        self.has_category_type(c, CategoryType::KANJI)
    }
    fn is_hiragana(&self, c: char) -> bool {
        self.has_category_type(c, CategoryType::HIRAGANA)
    }
    fn is_katakana(&self, c: char) -> bool {
        self.has_category_type(c, CategoryType::KATAKANA)
    }

    fn append_range(s: &mut String, r: Range<u32>) {
        if r.end != 0 {
            if r.len() == 1 {
                write!(s, "\\u{{{:X}}}", r.start).expect("should not fail");
            } else {
                write!(s, "\\u{{{:X}}}-\\u{{{:X}}}", r.start, r.end - 1).expect("should not fail");
            }
        }
    }

    fn append_class(&self, s: &mut String, t: CategoryType) {
        s.push('[');

        let mut cur_range = 0..0u32;

        for (r, c) in self.character_category.iter() {
            if c.intersects(t) {
                let r = (r.start as u32)..(r.end as u32);
                if cur_range.end == r.start {
                    cur_range = cur_range.start..r.end;
                    continue;
                }
                Self::append_range(s, cur_range);
                cur_range = r
            }
        }
        Self::append_range(s, cur_range);
        s.push(']')
    }

    fn kanji_pattern(&self) -> String {
        let mut s = String::with_capacity(40);
        self.append_class(&mut s, CategoryType::KANJI);
        s
    }

    fn reading_pattern(&self) -> String {
        let mut s = String::with_capacity(40);
        self.append_class(&mut s, CategoryType::HIRAGANA | CategoryType::KATAKANA);
        s
    }

    fn any_of_pattern<'a, I: Iterator<Item = &'a char>>(data: I) -> String {
        let mut s = String::with_capacity(40);
        s.push('[');
        for c in data {
            write!(s, "\\u{{{:X}}}", *c as u32).expect("should not fail")
        }
        s.push(']');
        s
    }

    fn make_regex(&self) -> SudachiResult<Regex> {
        let pattern = format!(
            "{kanji}({lbr}{reading}{{1,{count}}}{rbr})",
            kanji = self.kanji_pattern(),
            lbr = Self::any_of_pattern(self.left_bracket_set.iter()),
            reading = self.reading_pattern(),
            count = self.max_yomigana_length,
            rbr = Self::any_of_pattern(self.right_bracket_set.iter())
        );

        match Regex::new(&pattern) {
            Ok(r) => Ok(r),
            Err(e) => Err(SudachiError::PluginError(PluginError::InvalidDataFormat(
                e.to_string(),
            ))),
        }
    }
}

impl InputTextPlugin for IgnoreYomiganaPlugin {
    fn set_up(
        &mut self,
        settings: &Value,
        _config: &Config,
        grammar: &Grammar,
    ) -> SudachiResult<()> {
        let settings: PluginSettings = serde_json::from_value(settings.clone())?;

        let left_bracket_set = settings.leftBrackets.into_iter().collect();
        let right_bracket_set = settings.rightBrackets.into_iter().collect();
        let max_yomigana_length = settings.maxYomiganaLength;

        self.character_category = grammar.character_category.clone();
        self.left_bracket_set = left_bracket_set;
        self.right_bracket_set = right_bracket_set;
        self.max_yomigana_length = max_yomigana_length;

        self.regex = Some(self.make_regex()?);

        Ok(())
    }

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

    fn rewrite2<'a>(
        &'a self,
        input: &InputBuffer,
        mut edit: EditInput<'a>,
    ) -> SudachiResult<EditInput<'a>> {
        let regex = self.regex.as_ref().unwrap();

        let data = input.current();
        for m in regex.captures_iter(data) {
            let grp = m.get(1).unwrap(); //must be here
            edit.replace_ref(grp.range(), "");
        }

        Ok(edit)
    }
}
