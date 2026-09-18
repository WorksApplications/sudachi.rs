/*
 * Copyright (c) 2021-2026 Works Applications Co., Ltd.
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
use crate::plugin::PluginError;
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
        let settings: PluginSettings =
            serde_json::from_value(settings.clone()).map_err(PluginError::from)?;
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
    use crate::dic::connect::ConnectionMatrix;
    use crate::dic::pos::PosList;

    #[test]
    fn edit() {
        let left = 0;
        let right = 0;
        let bytes = build_mock_connection_bytes();
        let mut grammar = build_mock_grammar(&bytes);
        let plugin = InhibitConnectionPlugin {
            inhibit_pairs: vec![(left, right)],
        };

        plugin.edit(&mut grammar);
        assert_eq!(
            Grammar::INHIBITED_CONNECTION,
            grammar.connect_cost(left, right)
        );
    }

    fn build_mock_connection_bytes() -> Vec<u8> {
        let mut buf = Vec::new();
        // 1x1 connection with 0 element
        buf.extend(&1_i16.to_le_bytes());
        buf.extend(&1_i16.to_le_bytes());
        buf.extend(&0_i16.to_le_bytes());
        buf
    }

    fn build_mock_grammar(connection_bytes: &[u8]) -> Grammar<'_> {
        let pos_list = PosList::default();
        let connection = ConnectionMatrix::from_bytes(connection_bytes)
            .expect("Failed to parse connection matrix");
        Grammar::from_parts(pos_list, connection)
    }
}
