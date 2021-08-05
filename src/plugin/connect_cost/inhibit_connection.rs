use crate::dic::grammar::Grammar;
use crate::plugin::connect_cost::EditConnectionCostPlugin;
use crate::prelude::*;

pub struct InhibitConnectionPlugin {
    inhibit_pairs: Vec<(i16, i16)>,
}

impl InhibitConnectionPlugin {
    pub fn new() -> SudachiResult<InhibitConnectionPlugin> {
        // todo load from config

        // let sample = [(0, 233), (435, 332)];
        let inhibit_pairs = [].iter().map(|v| *v).collect();

        Ok(InhibitConnectionPlugin { inhibit_pairs })
    }

    fn inhibit_connection(grammar: &mut Grammar, left: i16, right: i16) {
        grammar.set_connect_cost(left, right, Grammar::INHIBITED_CONNECTION);
    }
}

impl EditConnectionCostPlugin for InhibitConnectionPlugin {
    fn edit(&self, grammar: &mut Grammar) {
        for (left, right) in &self.inhibit_pairs {
            InhibitConnectionPlugin::inhibit_connection(grammar, *left, *right);
        }
    }
}
