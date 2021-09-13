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
use crate::input_text::Utf8InputText;
use crate::lattice::node::Node;
use crate::prelude::*;

/// Trait of plugin to provide oov node during tokenization
pub trait OovProviderPlugin {
    /// Loads necessary information for the plugin
    fn set_up(&mut self, settings: &Value, config: &Config, grammar: &Grammar)
        -> SudachiResult<()>;

    /// Generate a list of oov nodes
    fn provide_oov(
        &self,
        input_text: &Utf8InputText,
        offset: usize,
        has_other_words: bool,
    ) -> SudachiResult<Vec<Node>>;

    /// Fills range field of generated nodes and returns.
    fn get_oov(
        &self,
        input_text: &Utf8InputText,
        offset: usize,
        has_other_words: bool,
    ) -> SudachiResult<Vec<Node>> {
        let mut nodes = self.provide_oov(input_text, offset, has_other_words)?;
        for node in &mut nodes {
            let length = node.word_info.as_ref().unwrap().head_word_length as usize;
            node.set_range(offset, offset + length);
        }
        Ok(nodes)
    }
}

/// Declare a plugin type and its constructor.
///
/// # Notes
/// This works by automatically generating an `extern "C"` function with a
/// pre-defined signature and symbol name. Therefore you will only be able to
/// declare one plugin per library.
#[macro_export]
macro_rules! declare_oov_provider_plugin {
    ($plugin_type:ty, $constructor:path) => {
        #[no_mangle]
        pub extern "C" fn load_plugin() -> *mut (dyn OovProviderPlugin + Sync) {
            // make sure the constructor is the correct type.
            let constructor: fn() -> $plugin_type = $constructor;

            let object = constructor();
            let boxed: Box<dyn OovProviderPlugin + Sync> = Box::new(object);
            Box::into_raw(boxed)
        }
    };
}

/// Plugin manager to handle multiple plugins
#[derive(Default)]
pub struct OovProviderPluginManager {
    plugins: Vec<Box<dyn OovProviderPlugin + Sync>>,
    libraries: Vec<Library>,
}
impl OovProviderPluginManager {
    pub fn load(
        &mut self,
        path: &Path,
        settings: &Value,
        config: &Config,
        grammar: &Grammar,
    ) -> SudachiResult<()> {
        type PluginCreate = unsafe fn() -> *mut (dyn OovProviderPlugin + Sync);

        let lib = unsafe { Library::new(path) }?;
        let load_plugin: Symbol<PluginCreate> = unsafe { lib.get(b"load_plugin") }?;
        let mut plugin = unsafe { Box::from_raw(load_plugin()) };
        plugin.set_up(settings, config, grammar)?;

        self.plugins.push(plugin);
        self.libraries.push(lib);
        Ok(())
    }

    pub fn plugins(&self) -> &[Box<dyn OovProviderPlugin + Sync>] {
        &self.plugins
    }

    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }
}
impl Drop for OovProviderPluginManager {
    fn drop(&mut self) {
        // Plugin drop must be called before Library drop.
        self.plugins.clear();
        self.libraries.clear();
    }
}

/// Load plugins based on config data
pub fn get_oov_plugins(
    config: &Config,
    grammar: &Grammar,
) -> SudachiResult<OovProviderPluginManager> {
    let mut manager = OovProviderPluginManager::default();

    for plugin in &config.oov_provider_plugins {
        let lib = super::get_plugin_path(plugin, config)?;
        manager.load(lib.as_path(), plugin, config, grammar)?;
    }

    Ok(manager)
}
