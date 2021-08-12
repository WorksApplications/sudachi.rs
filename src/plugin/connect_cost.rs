use std::path::{Path, PathBuf};

use libloading::{Library, Symbol};

use super::PluginError;
use crate::dic::grammar::Grammar;
use crate::prelude::*;

pub trait EditConnectionCostPlugin {
    fn set_up(&mut self, grammar: &Grammar) -> SudachiResult<()>;
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
    pub fn load(&mut self, path: &Path) -> Result<(), PluginError> {
        type PluginCreate = unsafe fn() -> *mut dyn EditConnectionCostPlugin;

        let lib = unsafe { Library::new(path) }?;
        let load_plugin: Symbol<PluginCreate> = unsafe { lib.get(b"load_plugin") }?;
        let plugin = unsafe { Box::from_raw(load_plugin()) };

        self.plugins.push(plugin);
        self.libraries.push(lib);

        Ok(())
    }

    pub fn set_up(&mut self, grammar: &Grammar) -> SudachiResult<()> {
        for plugin in &mut self.plugins {
            plugin.set_up(grammar)?;
        }
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
    grammar: &Grammar,
) -> SudachiResult<EditConnectionCostPluginManager> {
    // todo load from config
    let mut manager = EditConnectionCostPluginManager::default();

    manager.load(&PathBuf::from("./target/debug/libinhibit_connection.so"))?;

    manager.set_up(grammar)?;
    Ok(manager)
}
