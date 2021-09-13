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

use libloading::{Library, Symbol};
use serde_json::Value;
use std::path::Path;

use crate::config::Config;
use crate::dic::grammar::Grammar;
use crate::dic::lexicon::word_infos::WordInfo;
use crate::input_text::utf8_input_text::Utf8InputText;
use crate::lattice::{node::Node, Lattice};
use crate::prelude::*;

/// Trait of plugin to rewrite the path from lattice
pub trait PathRewritePlugin {
    /// Loads necessary information for the plugin
    fn set_up(&mut self, settings: &Value, config: &Config, grammar: &Grammar)
        -> SudachiResult<()>;

    /// Returns a rewrited path
    fn rewrite(
        &self,
        text: &Utf8InputText,
        path: Vec<Node>,
        lattice: &Lattice,
    ) -> SudachiResult<Vec<Node>>;

    /// Concatenate the nodes in the range and replace normalized_form if geven.
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

/// Declare a plugin type and its constructor.
///
/// # Notes
/// This works by automatically generating an `extern "C"` function with a
/// pre-defined signature and symbol name. Therefore you will only be able to
/// declare one plugin per library.
#[macro_export]
macro_rules! declare_path_rewrite_plugin {
    ($plugin_type:ty, $constructor:path) => {
        #[no_mangle]
        pub extern "C" fn load_plugin() -> *mut (dyn PathRewritePlugin + Sync) {
            // make sure the constructor is the correct type.
            let constructor: fn() -> $plugin_type = $constructor;

            let object = constructor();
            let boxed: Box<dyn PathRewritePlugin + Sync> = Box::new(object);
            Box::into_raw(boxed)
        }
    };
}

/// Plugin manager to handle multiple plugins
#[derive(Default)]
pub struct PathRewritePluginManager {
    plugins: Vec<Box<dyn PathRewritePlugin + Sync>>,
    libraries: Vec<Library>,
}
impl PathRewritePluginManager {
    pub fn load(
        &mut self,
        path: &Path,
        settings: &Value,
        config: &Config,
        grammar: &Grammar,
    ) -> SudachiResult<()> {
        type PluginCreate = unsafe fn() -> *mut (dyn PathRewritePlugin + Sync);

        let lib = unsafe { Library::new(path) }?;
        let load_plugin: Symbol<PluginCreate> = unsafe { lib.get(b"load_plugin") }?;
        let mut plugin = unsafe { Box::from_raw(load_plugin()) };
        plugin.set_up(settings, config, grammar)?;

        self.plugins.push(plugin);
        self.libraries.push(lib);
        Ok(())
    }

    pub fn plugins(&self) -> &[Box<dyn PathRewritePlugin + Sync>] {
        &self.plugins
    }

    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }
}
impl Drop for PathRewritePluginManager {
    fn drop(&mut self) {
        // Plugin drop must be called before Library drop.
        self.plugins.clear();
        self.libraries.clear();
    }
}

/// Load plugins based on config data
pub fn get_path_rewrite_plugins(
    config: &Config,
    grammar: &Grammar,
) -> SudachiResult<PathRewritePluginManager> {
    let mut manager = PathRewritePluginManager::default();

    for plugin in &config.path_rewrite_plugins {
        let lib = super::get_plugin_path(plugin, config)?;
        manager.load(lib.as_path(), plugin, config, grammar)?;
    }

    Ok(manager)
}
