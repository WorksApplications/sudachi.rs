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

mod ignore_yomigana;
pub mod default_input_text;
mod prolonged_sound_mark;

use serde_json::Value;

use crate::config::Config;
use crate::dic::grammar::Grammar;
use crate::input_text::Utf8InputTextBuilder;
use crate::plugin::input_text::ignore_yomigana::IgnoreYomiganaPlugin;
use crate::plugin::loader::PluginCategory;
use crate::prelude::*;
use crate::plugin::input_text::default_input_text::DefaultInputTextPlugin;
use crate::plugin::input_text::prolonged_sound_mark::ProlongedSoundMarkPlugin;

/// Trait of plugin to modify the input text before tokenization
pub trait InputTextPlugin: Sync {
    /// Loads necessary information for the plugin
    fn set_up(&mut self, settings: &Value, config: &Config, grammar: &Grammar)
        -> SudachiResult<()>;

    /// Rewrites input text
    ///
    /// builder::replace will be used inside
    fn rewrite(&self, builder: &mut Utf8InputTextBuilder);
}

impl PluginCategory for dyn InputTextPlugin {
    type BoxType = Box<dyn InputTextPlugin + Sync>;
    type InitFnType = unsafe extern "Rust" fn() -> SudachiResult<Self::BoxType>;
    fn configurations(cfg: &Config) -> &[Value] {
        &cfg.input_text_plugins
    }

    fn bundled_impl(name: &str) -> Option<Self::BoxType> {
        match name {
            "IgnoreYomiganaPlugin" => {
                Some(Box::new(IgnoreYomiganaPlugin::default()))
            }
            "DefaultInputTextPlugin" => {
                Some(Box::new(DefaultInputTextPlugin::default()))
            }
            "ProlongedSoundMarkPlugin" => {
                Some(Box::new(ProlongedSoundMarkPlugin::default()))
            }
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