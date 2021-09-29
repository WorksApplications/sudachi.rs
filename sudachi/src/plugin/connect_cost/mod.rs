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

mod inhibit_connection;

use serde_json::Value;

use crate::config::Config;
use crate::dic::grammar::Grammar;
use crate::plugin::connect_cost::inhibit_connection::InhibitConnectionPlugin;
use crate::plugin::PluginCategory;
use crate::prelude::*;

/// Trait of plugin to edit connection cost in the grammar
pub trait EditConnectionCostPlugin: Sync + Send {
    /// Loads necessary information for the plugin
    fn set_up(&mut self, settings: &Value, config: &Config, grammar: &Grammar)
        -> SudachiResult<()>;

    /// Edits the grammar
    fn edit(&self, grammar: &mut Grammar);
}

impl PluginCategory for dyn EditConnectionCostPlugin {
    type BoxType = Box<dyn EditConnectionCostPlugin + Sync + Send>;
    type InitFnType = unsafe fn() -> SudachiResult<Self::BoxType>;

    fn configurations(cfg: &Config) -> &[Value] {
        &cfg.connection_cost_plugins
    }

    fn bundled_impl(name: &str) -> Option<Self::BoxType> {
        match name {
            "InhibitConnectionPlugin" => Some(Box::new(InhibitConnectionPlugin::default())),
            _ => None,
        }
    }

    fn do_setup(
        ptr: &mut Self::BoxType,
        settings: &Value,
        config: &Config,
        grammar: &Grammar,
    ) -> SudachiResult<()> {
        ptr.set_up(settings, config, grammar)
    }
}
