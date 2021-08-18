use std::path::Path;

use libloading::{Library, Symbol};
use serde_json::Value;

use crate::config::Config;
use crate::dic::grammar::Grammar;
use crate::prelude::*;

pub trait EditConnectionCostPlugin {
    fn set_up(&mut self, settings: &Value, config: &Config, grammar: &Grammar)
        -> SudachiResult<()>;
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
        pub extern "C" fn load_plugin() -> *mut EditConnectionCostPlugin {
            // make sure the constructor is the correct type.
            let constructor: fn() -> $plugin_type = $constructor;

            let object = constructor();
            let boxed: Box<EditConnectionCostPlugin> = Box::new(object);
            Box::into_raw(boxed)
        }
    };
}

#[derive(Default)]
pub struct EditConnectionCostPluginManager {
    plugins: Vec<Box<dyn EditConnectionCostPlugin>>,
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
        type PluginCreate = unsafe fn() -> *mut dyn EditConnectionCostPlugin;

        let lib = unsafe { Library::new(path) }?;
        let load_plugin: Symbol<PluginCreate> = unsafe { lib.get(b"load_plugin") }?;
        let mut plugin = unsafe { Box::from_raw(load_plugin()) };
        plugin.set_up(settings, config, grammar)?;

        self.plugins.push(plugin);
        self.libraries.push(lib);
        Ok(())
    }

    pub fn plugins(&self) -> &[Box<dyn EditConnectionCostPlugin>] {
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
