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

use crate::analysis::created::CreatedWords;
use crate::analysis::Node;
use itertools::Itertools;
use serde::Deserialize;
use serde_json::Value;
use std::fmt::Display;

use crate::config::Config;
use crate::dic::grammar::Grammar;
use crate::input_text::InputBuffer;
use crate::plugin::oov::mecab_oov::MeCabOovPlugin;
use crate::plugin::oov::simple_oov::SimpleOovPlugin;
use crate::plugin::PluginCategory;
use crate::prelude::*;

pub mod mecab_oov;
pub mod simple_oov;

#[derive(Eq, PartialEq, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
enum UserPosMode {
    Allow,
    Forbid,
}

impl Default for UserPosMode {
    fn default() -> Self {
        UserPosMode::Forbid
    }
}

/// Trait of plugin to provide oov node during tokenization
pub trait OovProviderPlugin: Sync + Send {
    /// Loads necessary information for the plugin
    fn set_up(
        &mut self,
        settings: &Value,
        config: &Config,
        grammar: &mut Grammar,
    ) -> SudachiResult<()>;

    /// Generate a list of oov nodes
    /// offset - char idx
    fn provide_oov(
        &self,
        input_text: &InputBuffer,
        offset: usize,
        other_words: CreatedWords,
        result: &mut Vec<Node>,
    ) -> SudachiResult<usize>;
}

impl PluginCategory for dyn OovProviderPlugin {
    type BoxType = Box<dyn OovProviderPlugin + Sync + Send>;
    type InitFnType = unsafe fn() -> SudachiResult<Self::BoxType>;
    fn configurations(cfg: &Config) -> &[Value] {
        &cfg.oov_provider_plugins
    }

    fn bundled_impl(name: &str) -> Option<Self::BoxType> {
        match name {
            "SimpleOovPlugin" => Some(Box::new(SimpleOovPlugin::default())),
            "MeCabOovPlugin" => Some(Box::new(MeCabOovPlugin::default())),
            _ => None,
        }
    }

    fn do_setup(
        ptr: &mut Self::BoxType,
        settings: &Value,
        config: &Config,
        grammar: &mut Grammar,
    ) -> SudachiResult<()> {
        ptr.set_up(settings, config, grammar)
    }
}

trait UserPosSupport {
    fn maybe_user_pos<S: AsRef<str> + ToString + Display>(
        &mut self,
        pos: &[S],
        mode: UserPosMode,
    ) -> SudachiResult<u16>;
}

impl<'a> UserPosSupport for &'a mut Grammar<'_> {
    fn maybe_user_pos<S: AsRef<str> + ToString + Display>(
        &mut self,
        pos: &[S],
        mode: UserPosMode,
    ) -> SudachiResult<u16> {
        if let Some(id) = self.get_part_of_speech_id(pos) {
            return Ok(id);
        }

        match mode {
            UserPosMode::Allow => self.register_pos(pos),
            UserPosMode::Forbid => Err(SudachiError::InvalidPartOfSpeech(format!(
                "POS {} was not in the dictionary, user-defined POS are forbidden",
                pos.iter().join(",")
            ))),
        }
    }
}
