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

use crate::config::Config;
use crate::dic::grammar::Grammar;
use crate::input_text::{InputBuffer, InputEditor};
use crate::plugin::input_text::InputTextPlugin;
use crate::plugin::PluginError;
use crate::prelude::*;

#[cfg(test)]
mod tests;

/// Replace (consecutive) prolonged sound mark by one symbol.
#[derive(Default)]
pub struct ProlongedSoundMarkPlugin {
    psm_set: HashSet<char>,
    replace_symbol: String,
    regex: Option<Regex>,
}

/// Struct corresponds with raw config json file.
#[allow(non_snake_case)]
#[derive(Deserialize)]
struct PluginSettings {
    prolongedSoundMarks: Vec<char>,
    replacementSymbol: Option<String>,
}

impl ProlongedSoundMarkPlugin {
    /// Convert prolongation marks to a Regex which will match at least two patterns
    fn prolongs_as_regex<I: Iterator<Item = char>>(data: I) -> SudachiResult<Regex> {
        let mut pattern = String::with_capacity(32);
        pattern.push('[');
        for symbol in data {
            match symbol {
                '-' | '[' | ']' | '^' | '\\' => {
                    write!(pattern, "\\u{{{:X}}}", symbol as u32).expect("should not happen")
                }
                c => pattern.push(c),
            }
        }
        pattern.push_str("]{2,}");
        match Regex::new(&pattern) {
            Ok(re) => Ok(re),
            Err(e) => Err(SudachiError::PluginError(PluginError::InvalidDataFormat(
                e.to_string(),
            ))),
        }
    }
}

impl InputTextPlugin for ProlongedSoundMarkPlugin {
    fn set_up(
        &mut self,
        settings: &Value,
        _config: &Config,
        _grammar: &Grammar,
    ) -> SudachiResult<()> {
        let settings: PluginSettings = serde_json::from_value(settings.clone())?;

        let psm_set = settings.prolongedSoundMarks.into_iter().collect();
        let replace_symbol = settings.replacementSymbol;

        self.psm_set = psm_set;
        self.replace_symbol = replace_symbol.unwrap_or("ー".to_string());
        self.regex = Some(Self::prolongs_as_regex(self.psm_set.iter().cloned())?);
        Ok(())
    }

    fn rewrite_impl<'a>(
        &'a self,
        input: &InputBuffer,
        mut edit: InputEditor<'a>,
    ) -> SudachiResult<InputEditor<'a>> {
        let re = self.regex.as_ref().unwrap();
        let data = input.current();

        for m in re.find_iter(data) {
            edit.replace_ref(m.range(), &self.replace_symbol)
        }
        Ok(edit)
    }
}
