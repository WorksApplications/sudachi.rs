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
use crate::prelude::*;

/// Trait of plugin to edit connection cost in the grammar
pub trait EditConnectionCostPlugin: Sync + Send {
    /// Loads necessary information for the plugin
    fn set_up(&mut self, settings: &Value, config: &Config, grammar: &Grammar)
        -> SudachiResult<()>;

    /// Edits the grammar
    fn edit(&self, grammar: &mut Grammar);
}

/// Declare a plugin type and its constructor.
///
/// # Notes
/// This works by automatically generating an `extern "C"` function with a
/// pre-defined signature and symbol name. Therefore you will only be able to
/// declare one plugin per library.
#[macro_export]
macro_rules! declare_connect_cost_plugin {
    ($plugin_type:ty, $constructor:path) => {
        #[no_mangle]
        pub extern "C" fn load_plugin() -> *mut (dyn EditConnectionCostPlugin + Sync) {
            // make sure the constructor is the correct type.
            let constructor: fn() -> $plugin_type = $constructor;

            let object = constructor();
            let boxed: Box<dyn EditConnectionCostPlugin + Sync> = Box::new(object);
            Box::into_raw(boxed)
        }
    };
}

/// Plugin manager to handle multiple plugins
#[derive(Default)]
pub struct EditConnectionCostPluginManager {
    plugins: Vec<Box<dyn EditConnectionCostPlugin + Sync>>,
    libraries: Vec<Library>,
}
impl EditConnectionCostPluginManager {
    pub fn load(
        &mut self,
        path: &Path,
        settings: &Value,
        config: &Config,
        grammar: &Grammar,
    ) -> SudachiResult<()> {
        type PluginCreate = unsafe fn() -> *mut (dyn EditConnectionCostPlugin + Sync);
        let lib = unsafe { Library::new(path) }?;
        let load_plugin: Symbol<PluginCreate> = unsafe { lib.get(b"load_plugin") }?;
        let mut plugin = unsafe { Box::from_raw(load_plugin()) };
        plugin.set_up(settings, config, grammar)?;

        self.plugins.push(plugin);
        self.libraries.push(lib);
        Ok(())
    }

    pub fn plugins(&self) -> &[Box<dyn EditConnectionCostPlugin + Sync>] {
        &self.plugins
    }

    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }
}
impl Drop for EditConnectionCostPluginManager {
    fn drop(&mut self) {
        // Plugin drop must be called before Library drop.
        self.plugins.clear();
        self.libraries.clear();
    }
}

/// Load plugins based on config data
pub fn get_edit_connection_cost_plugins(
    config: &Config,
    grammar: &Grammar,
) -> SudachiResult<EditConnectionCostPluginManager> {
    let mut manager = EditConnectionCostPluginManager::default();

    for plugin in &config.connection_cost_plugins {
        let lib = super::get_plugin_path(plugin, config)?;
        manager.load(lib.as_path(), plugin, config, grammar)?;
    }

    Ok(manager)
}
