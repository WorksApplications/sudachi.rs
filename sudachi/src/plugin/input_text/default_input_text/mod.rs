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

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};
use serde::Deserialize;
use serde_json::Value;
use unicode_normalization::{is_nfkc_quick, IsNormalized, UnicodeNormalization};

use crate::config::Config;
use crate::dic::grammar::Grammar;
use crate::hash::RoMu;
use crate::input_text::{InputBuffer, InputEditor};
use crate::plugin::input_text::InputTextPlugin;
use crate::prelude::*;

#[cfg(test)]
mod tests;

const DEFAULT_REWRITE_DEF_FILE: &str = "rewrite.def";

/// Provides basic normalization of the input text
#[derive(Default)]
pub struct DefaultInputTextPlugin {
    /// Set of characters to skip normalization
    ignore_normalize_set: HashSet<char, RoMu>,
    /// Mapping from a character to the maximum char_length of possible replacement
    key_lengths: HashMap<char, usize>,
    /// Replacement mapping
    replace_char_map: HashMap<String, String>,
    /// Checks whether the full string contains symbols to normalize
    full_checker: Option<AhoCorasick>,
    /// Checks the same as previous, but checks only prefix
    anchored_checker: Option<AhoCorasick>,
    replacements: Vec<String>,
}

/// Struct corresponds with raw config json file.
#[allow(non_snake_case)]
#[derive(Deserialize)]
struct PluginSettings {
    rewriteDef: Option<PathBuf>,
}

impl DefaultInputTextPlugin {
    /// Loads rewrite definition
    ///
    /// Definition syntax:
    ///     Ignored normalize:
    ///         Each line contains a character
    ///     Replace char list:
    ///         Each line contains two strings separated by white spaces
    ///         Plugin replaces the first by the second
    ///         Same target string cannot be defined multiple times
    ///     Empty or line starts with "#" will be ignored
    fn read_rewrite_lists<T: BufRead>(&mut self, reader: T) -> SudachiResult<()> {
        let mut ignore_normalize_set = HashSet::with_hasher(RoMu::new());
        let mut key_lengths = HashMap::new();
        let mut replace_char_map = HashMap::new();
        for (i, line) in reader.lines().enumerate() {
            let line = line?;
            let line = line.trim();
            if line.is_empty() || line.chars().next().unwrap() == '#' {
                continue;
            }
            let cols: Vec<_> = line.split_whitespace().collect();

            // ignored normalize list
            if cols.len() == 1 {
                if cols[0].chars().count() != 1 {
                    return Err(SudachiError::InvalidDataFormat(
                        i,
                        format!("{} is not character", cols[0]),
                    ));
                }
                ignore_normalize_set.insert(cols[0].chars().next().unwrap());
                continue;
            }
            // replace char list
            if cols.len() == 2 {
                if replace_char_map.contains_key(cols[0]) {
                    return Err(SudachiError::InvalidDataFormat(
                        i,
                        format!("{} is already defined", cols[0]),
                    ));
                }
                let first_char = cols[0].chars().next().unwrap();
                let n_char = cols[0].chars().count();
                if key_lengths.get(&first_char).map(|v| *v).unwrap_or(0) < n_char {
                    key_lengths.insert(first_char, n_char);
                }
                replace_char_map.insert(cols[0].to_string(), cols[1].to_string());
                continue;
            }
            return Err(SudachiError::InvalidDataFormat(i, "".to_string()));
        }

        self.ignore_normalize_set = ignore_normalize_set;
        self.key_lengths = key_lengths;
        self.replace_char_map = replace_char_map;

        let mut values: Vec<String> = Vec::new();
        let mut keys: Vec<String> = Vec::new();

        for (k, v) in self.replace_char_map.iter() {
            keys.push(k.clone());
            values.push(v.clone());
        }

        self.full_checker = Some(
            AhoCorasickBuilder::new()
                .dfa(true)
                .match_kind(MatchKind::LeftmostLongest)
                .build(keys.clone()),
        );

        self.anchored_checker = Some(
            AhoCorasickBuilder::new()
                .dfa(true)
                .match_kind(MatchKind::LeftmostLongest)
                .anchored(true)
                .build(keys),
        );

        self.replacements = values;

        Ok(())
    }

    #[inline]
    fn should_ignore(&self, ch: char) -> bool {
        self.ignore_normalize_set.contains(&ch)
    }

    /// Fast case: lowercasing is not needed and the string is already in NFKC
    /// Use AhoCorasick automaton to find all replacements and replace them
    ///
    /// Ignores are not used here, forced replacements have higher priority
    /// Fast version does not need to walk every character!
    fn replace_fast<'a>(
        &'a self,
        buffer: &InputBuffer,
        mut replacer: InputEditor<'a>,
    ) -> SudachiResult<InputEditor<'a>> {
        let cur = buffer.current();
        let checker = self.full_checker.as_ref().unwrap();

        for m in checker.find_iter(cur) {
            let replacement = self.replacements.get(m.pattern()).unwrap();
            replacer.replace_ref(m.start()..m.end(), replacement);
        }

        Ok(replacer)
    }

    /// Slow case: need to handle lowercasing or NFKC normalization
    /// Slow version needs to walk every character
    fn replace_slow<'a>(
        &'a self,
        buffer: &InputBuffer,
        mut replacer: InputEditor<'a>,
    ) -> SudachiResult<InputEditor<'a>> {
        let cur = buffer.current();
        let checker = self.anchored_checker.as_ref().unwrap();
        let mut min_offset = 0;

        for (offset, ch) in cur.char_indices() {
            if offset < min_offset {
                continue;
            }
            // 1. replacement as defined by char.def
            if let Some(m) = checker.earliest_find(&cur[offset..]) {
                let range = offset..offset + m.end();
                let replacement = self.replacements[m.pattern()].as_str();
                min_offset = range.end;
                replacer.replace_ref(range, replacement);
                continue;
            }

            // 2. handle normalization
            let need_lowercase = ch.is_uppercase();
            let need_nkfc = !self.should_ignore(ch)
                && match is_nfkc_quick(std::iter::once(ch)) {
                    IsNormalized::Yes => false,
                    _ => true,
                };

            // iterator types are incompatible, so calls can't be moved outside branches
            match (need_lowercase, need_nkfc) {
                //no need to do anything
                (false, false) => continue,
                // only lowercasing
                (true, false) => {
                    let chars = ch.to_lowercase();
                    self.handle_normalization_slow(chars, &mut replacer, offset, ch.len_utf8(), ch)
                }
                // only normalization
                (false, true) => {
                    let chars = std::iter::once(ch).nfkc();
                    self.handle_normalization_slow(chars, &mut replacer, offset, ch.len_utf8(), ch)
                }
                // both
                (true, true) => {
                    let chars = ch.to_lowercase().nfkc();
                    self.handle_normalization_slow(chars, &mut replacer, offset, ch.len_utf8(), ch)
                }
            }
        }
        Ok(replacer)
    }

    fn handle_normalization_slow<'a, I: Iterator<Item = char>>(
        &'a self,
        mut data: I,
        replacer: &mut InputEditor<'a>,
        start: usize,
        len: usize,
        ch: char,
    ) {
        match data.next() {
            Some(ch2) => {
                if ch2 == ch {
                    return;
                }
                replacer.replace_char_iter(start..start + len, ch2, data)
            }
            None => return,
        }
    }
}

impl InputTextPlugin for DefaultInputTextPlugin {
    fn set_up(
        &mut self,
        settings: &Value,
        config: &Config,
        _grammar: &Grammar,
    ) -> SudachiResult<()> {
        let settings: PluginSettings = serde_json::from_value(settings.clone())?;

        let rewrite_file_path = config.complete_path(
            settings
                .rewriteDef
                .unwrap_or(PathBuf::from(DEFAULT_REWRITE_DEF_FILE)),
        );

        let reader = BufReader::new(fs::File::open(&rewrite_file_path)?);
        self.read_rewrite_lists(reader)?;

        Ok(())
    }

    fn uses_chars(&self) -> bool {
        true
    }

    fn rewrite_impl<'a>(
        &'a self,
        buffer: &InputBuffer,
        edit: InputEditor<'a>,
    ) -> SudachiResult<InputEditor<'a>> {
        let chars = buffer.current_chars();
        let need_nkfc = match is_nfkc_quick(chars.iter().cloned()) {
            IsNormalized::Yes => false,
            _ => true,
        };

        let need_lowercase = chars.iter().any(|c| c.is_uppercase());

        if need_nkfc || need_lowercase {
            self.replace_slow(buffer, edit)
        } else {
            self.replace_fast(buffer, edit)
        }
    }
}
