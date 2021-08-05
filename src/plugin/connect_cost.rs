use crate::dic::grammar::Grammar;
use crate::prelude::*;

pub mod inhibit_connection;

pub trait EditConnectionCostPlugin {
    fn edit(&self, grammar: &mut Grammar);
}

pub fn get_edit_connection_cost_plugins(
    _grammar: &Grammar,
) -> SudachiResult<Vec<Box<dyn EditConnectionCostPlugin>>> {
    // todo load from config
    let mut plugins: Vec<Box<dyn EditConnectionCostPlugin>> = vec![];

    plugins.push(Box::new(inhibit_connection::InhibitConnectionPlugin::new()?));

    Ok(plugins)
}
