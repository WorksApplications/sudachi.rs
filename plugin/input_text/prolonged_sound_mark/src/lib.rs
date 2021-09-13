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

use serde::Deserialize;
use serde_json::Value;
use std::collections::HashSet;

use sudachi::config::Config;
use sudachi::declare_input_text_plugin;
use sudachi::dic::grammar::Grammar;
use sudachi::input_text::Utf8InputTextBuilder;
use sudachi::plugin::input_text::InputTextPlugin;
use sudachi::prelude::*;

#[cfg(test)]
mod tests;

declare_input_text_plugin!(ProlongedSoundMarkPlugin, ProlongedSoundMarkPlugin::default);

/// Replace (consecutive) prolonged sound mark by one symbol.
#[derive(Default)]
pub struct ProlongedSoundMarkPlugin {
    psm_set: HashSet<char>,
    replace_symbol: String,
}

/// Struct corresponds with raw config json file.
#[allow(non_snake_case)]
#[derive(Deserialize)]
struct PluginSettings {
    prolongedSoundMarks: Vec<char>,
    replacementSymbol: Option<String>,
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

        Ok(())
    }

    fn rewrite(&self, builder: &mut Utf8InputTextBuilder) {
        let text = builder.modified.clone();
        let n = builder.modified.chars().count();
        let mut offset = 0;
        let mut is_psm = false;
        let mut m_start_idx = n;
        for (i, c) in text.chars().enumerate() {
            if !is_psm && self.psm_set.contains(&c) {
                is_psm = true;
                m_start_idx = i;
            } else if is_psm && !self.psm_set.contains(&c) {
                if i > m_start_idx + 1 {
                    builder.replace(m_start_idx - offset..i - offset, &self.replace_symbol);
                    offset += i - m_start_idx - 1;
                }
                is_psm = false;
            }
        }
        if is_psm && n > m_start_idx + 1 {
            builder.replace(m_start_idx - offset..n - offset, &self.replace_symbol);
        }
    }
}
