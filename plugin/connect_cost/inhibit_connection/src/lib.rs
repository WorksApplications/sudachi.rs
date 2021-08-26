use serde::Deserialize;
use serde_json::Value;

use sudachi::config::Config;
use sudachi::declare_connect_cost_plugin;
use sudachi::dic::grammar::Grammar;
use sudachi::plugin::connect_cost::EditConnectionCostPlugin;
use sudachi::prelude::*;

declare_connect_cost_plugin!(InhibitConnectionPlugin, InhibitConnectionPlugin::default);

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
