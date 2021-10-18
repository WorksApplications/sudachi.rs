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

pub mod join_katakana_oov;
pub mod join_numeric;

use serde_json::Value;

use crate::analysis::lattice::Lattice;
use crate::analysis::node::ResultNode;
use crate::config::Config;
use crate::dic::grammar::Grammar;
use crate::input_text::InputBuffer;
use crate::plugin::path_rewrite::join_katakana_oov::JoinKatakanaOovPlugin;
use crate::plugin::path_rewrite::join_numeric::JoinNumericPlugin;
use crate::plugin::PluginCategory;
use crate::prelude::*;

/// Trait of plugin to rewrite the path from lattice
pub trait PathRewritePlugin: Sync + Send {
    /// Loads necessary information for the plugin
    fn set_up(&mut self, settings: &Value, config: &Config, grammar: &Grammar)
        -> SudachiResult<()>;

    /// Returns a rewritten path
    fn rewrite(
        &self,
        text: &InputBuffer,
        path: Vec<ResultNode>,
        lattice: &Lattice,
    ) -> SudachiResult<Vec<ResultNode>>;
}

impl PluginCategory for dyn PathRewritePlugin {
    type BoxType = Box<dyn PathRewritePlugin + Sync + Send>;
    type InitFnType = unsafe fn() -> SudachiResult<Self::BoxType>;
    fn configurations(cfg: &Config) -> &[Value] {
        &cfg.path_rewrite_plugins
    }

    fn bundled_impl(name: &str) -> Option<Self::BoxType> {
        match name {
            "JoinNumericPlugin" => Some(Box::new(JoinNumericPlugin::default())),
            "JoinKatakanaOovPlugin" => Some(Box::new(JoinKatakanaOovPlugin::default())),
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
