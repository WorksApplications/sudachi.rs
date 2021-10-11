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

use crate::analysis::{lattice::Lattice, node::Node};
use crate::config::Config;
use crate::dic::grammar::Grammar;
use crate::dic::lexicon::word_infos::WordInfo;
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
        path: Vec<Node>,
        lattice: &Lattice,
    ) -> SudachiResult<Vec<Node>>;

    /// Concatenate the nodes in the range and replace normalized_form if given.
    fn concatenate(
        &self,
        mut path: Vec<Node>,
        begin: usize,
        end: usize,
        normalized_form: Option<String>,
    ) -> SudachiResult<Vec<Node>> {
        if begin >= end {
            return Err(SudachiError::InvalidRange(begin, end));
        }

        let b = path[begin].begin;
        let e = path[end - 1].end;
        let word_infos: Vec<_> = path[begin..end]
            .iter()
            .map(|node| node.word_info.as_ref())
            .collect::<Option<_>>()
            .ok_or(SudachiError::MissingWordInfo)?;
        let pos_id = word_infos[0].pos_id;
        let surface = word_infos
            .iter()
            .fold(String::new(), |acc, wi| acc + &wi.surface);
        let head_word_length = word_infos
            .iter()
            .fold(0, |acc, wi| acc + wi.head_word_length);
        let normalized_form = normalized_form.unwrap_or_else(|| {
            word_infos
                .iter()
                .fold(String::new(), |acc, wi| acc + &wi.normalized_form)
        });
        let reading_form = word_infos
            .iter()
            .fold(String::new(), |acc, wi| acc + &wi.reading_form);
        let dictionary_form = word_infos
            .iter()
            .fold(String::new(), |acc, wi| acc + &wi.dictionary_form);

        let mut node = Node::default();
        node.set_range(b, e);
        node.set_word_info(WordInfo {
            surface,
            head_word_length,
            pos_id,
            normalized_form,
            reading_form,
            dictionary_form,
            ..Default::default()
        });

        path[begin] = node;
        path.drain(begin + 1..end);
        Ok(path)
    }

    /// Concatenate the nodes in the range and set pos_id.
    fn concatenate_oov(
        &self,
        mut path: Vec<Node>,
        begin: usize,
        end: usize,
        pos_id: u16,
    ) -> SudachiResult<Vec<Node>> {
        if begin >= end {
            return Err(SudachiError::InvalidRange(begin, end));
        }

        let b = path[begin].begin;
        let e = path[end - 1].end;
        let word_infos: Vec<_> = path[begin..end]
            .iter()
            .map(|node| node.word_info.as_ref())
            .collect::<Option<_>>()
            .ok_or(SudachiError::MissingWordInfo)?;
        let surface = word_infos
            .iter()
            .fold(String::new(), |acc, wi| acc + &wi.surface);
        let head_word_length = word_infos
            .iter()
            .fold(0, |acc, wi| acc + wi.head_word_length);

        let mut node = Node::default();
        node.set_range(b, e);
        node.set_word_info(WordInfo {
            normalized_form: surface.clone(),
            dictionary_form: surface.clone(),
            surface,
            head_word_length,
            pos_id,
            ..Default::default()
        });

        path[begin] = node;
        path.drain(begin + 1..end);
        Ok(path)
    }
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
