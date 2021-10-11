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

pub mod default_input_text;
mod ignore_yomigana;
mod prolonged_sound_mark;

use serde_json::Value;

use crate::config::Config;
use crate::dic::grammar::Grammar;
use crate::input_text::{InputBuffer, InputEditor};
use crate::plugin::input_text::default_input_text::DefaultInputTextPlugin;
use crate::plugin::input_text::ignore_yomigana::IgnoreYomiganaPlugin;
use crate::plugin::input_text::prolonged_sound_mark::ProlongedSoundMarkPlugin;
use crate::plugin::loader::PluginCategory;
use crate::prelude::*;

/// Trait of plugin to modify the input text before tokenization
pub trait InputTextPlugin: Sync + Send {
    /// Loads necessary information for the plugin
    fn set_up(&mut self, settings: &Value, config: &Config, grammar: &Grammar)
        -> SudachiResult<()>;

    /// Whether the rewrite process uses chars
    fn uses_chars(&self) -> bool {
        false
    }

    /// Perform rewrites
    fn rewrite(&self, input: &mut InputBuffer) -> SudachiResult<()> {
        if self.uses_chars() {
            input.refresh_chars()
        }
        input.with_editor(|b, r| {
            // deprecation is to discourage calling the work function
            #[allow(deprecated)]
            self.rewrite_impl(b, r)
        })
    }

    /// Actual implementation of rewriting. Call `apply_rewrite` instead.
    #[deprecated(note = "call apply_rewrite instead")]
    fn rewrite_impl<'a>(
        &'a self,
        input: &InputBuffer,
        edit: InputEditor<'a>,
    ) -> SudachiResult<InputEditor<'a>>;
}

impl PluginCategory for dyn InputTextPlugin {
    type BoxType = Box<dyn InputTextPlugin + Sync + Send>;
    type InitFnType = unsafe fn() -> SudachiResult<Self::BoxType>;
    fn configurations(cfg: &Config) -> &[Value] {
        &cfg.input_text_plugins
    }

    fn bundled_impl(name: &str) -> Option<Self::BoxType> {
        match name {
            "IgnoreYomiganaPlugin" => Some(Box::new(IgnoreYomiganaPlugin::default())),
            "DefaultInputTextPlugin" => Some(Box::new(DefaultInputTextPlugin::default())),
            "ProlongedSoundMarkPlugin" => Some(Box::new(ProlongedSoundMarkPlugin::default())),
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
