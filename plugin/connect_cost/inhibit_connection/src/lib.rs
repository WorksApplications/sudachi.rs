use sudachi::declare_connect_cost_plugin;
use sudachi::dic::grammar::Grammar;
use sudachi::plugin::connect_cost::EditConnectionCostPlugin;
use sudachi::prelude::*;

declare_connect_cost_plugin!(InhibitConnectionPlugin, InhibitConnectionPlugin::default);

#[derive(Default)]
pub struct InhibitConnectionPlugin {
    inhibit_pairs: Vec<(i16, i16)>,
}

impl InhibitConnectionPlugin {
    fn inhibit_connection(grammar: &mut Grammar, left: i16, right: i16) {
        grammar.set_connect_cost(left, right, Grammar::INHIBITED_CONNECTION);
    }
}

impl EditConnectionCostPlugin for InhibitConnectionPlugin {
    fn set_up(&mut self, _grammar: &Grammar) -> SudachiResult<()> {
        // todo load from config

        // let sample = [(0, 233), (435, 332)];
        let inhibit_pairs = [].iter().map(|v| *v).collect();

        self.inhibit_pairs = inhibit_pairs;

        Ok(())
    }

    fn edit(&self, grammar: &mut Grammar) {
        for (left, right) in &self.inhibit_pairs {
            InhibitConnectionPlugin::inhibit_connection(grammar, *left, *right);
        }
    }
}
