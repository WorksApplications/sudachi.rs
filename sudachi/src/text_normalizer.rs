/*
 * Copyright (c) 2026 Works Applications Co., Ltd.
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

use crate::config::ConfigBuilder;
use crate::dic::grammar::Grammar;
use crate::input_text::InputBuffer;
use crate::plugin::input_text::default_input_text::DefaultInputTextPlugin;
use crate::plugin::input_text::InputTextPlugin;
use crate::prelude::*;

/// Applies the same default input-text normalization used by tokenizer input processing.
pub struct TextNormalizer {
    plugin: DefaultInputTextPlugin,
}

impl TextNormalizer {
    pub fn new(grammar: &Grammar) -> SudachiResult<Self> {
        let mut plugin = DefaultInputTextPlugin::default();
        let cfg = ConfigBuilder::empty().push_embedded().build();
        plugin.set_up(
            &serde_json::Value::Object(serde_json::Map::default()),
            &cfg,
            grammar,
        )?;
        Ok(Self { plugin })
    }

    pub fn normalize(&self, text: &str) -> SudachiResult<String> {
        let mut input = InputBuffer::from(text);
        self.plugin.rewrite(&mut input)?;
        Ok(input.current().to_string())
    }
}
