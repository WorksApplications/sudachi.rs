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

use crate::config::Config;
use crate::dic::grammar::Grammar;
use crate::plugin::connect_cost::EditConnectionCostPlugin;
use crate::prelude::*;

/// A edit connection cost plugin for inhibiting the connections.
///
/// Example setting file
/// ``
/// {
///     {
///         "class": "relative-path/to/so-file/from/resource-path",
///         "inhibitPair": [[0, 233], [435, 332]]
///     }
/// }
/// ``
#[derive(Default)]
pub struct InhibitConnectionPlugin {
    /// At each pair, the first one is right_id of the left node
    /// and the second one is left_id of right node in a connection
    inhibit_pairs: Vec<(i16, i16)>,
}

/// Struct corresponds with raw config json file.
#[allow(non_snake_case)]
#[derive(Deserialize)]
struct PluginSettings {
    inhibitPair: Vec<(i16, i16)>,
}

impl InhibitConnectionPlugin {
    fn inhibit_connection(grammar: &mut Grammar, left: i16, right: i16) {
        grammar.set_connect_cost(left, right, Grammar::INHIBITED_CONNECTION);
    }
}

impl EditConnectionCostPlugin for InhibitConnectionPlugin {
    fn set_up(
        &mut self,
        settings: &Value,
        _config: &Config,
        _grammar: &Grammar,
    ) -> SudachiResult<()> {
        let settings: PluginSettings = serde_json::from_value(settings.clone())?;
        let inhibit_pairs = settings.inhibitPair;
        self.inhibit_pairs = inhibit_pairs;
        Ok(())
    }

    fn edit(&self, grammar: &mut Grammar) {
        for (left, right) in &self.inhibit_pairs {
            InhibitConnectionPlugin::inhibit_connection(grammar, *left, *right);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edit() {
        let left = 0;
        let right = 0;
        let bytes = build_mock_bytes();
        let mut grammar = build_mock_grammar(&bytes);
        let mut plugin = InhibitConnectionPlugin::default();
        plugin.inhibit_pairs = vec![(left, right)];

        plugin.edit(&mut grammar);
        assert_eq!(
            Grammar::INHIBITED_CONNECTION,
            grammar.connect_cost(left, right)
        );
    }

    fn build_mock_bytes() -> Vec<u8> {
        let mut buf = Vec::new();
        // 0 - pos size, 1x1 connection with 0 element
        buf.extend(&(0 as i16).to_le_bytes());
        buf.extend(&(1 as i16).to_le_bytes());
        buf.extend(&(1 as i16).to_le_bytes());
        buf.extend(&(0 as i16).to_le_bytes());
        buf
    }

    fn build_mock_grammar(bytes: &[u8]) -> Grammar {
        Grammar::parse(bytes, 0).expect("Failed to create grammar")
    }
}
