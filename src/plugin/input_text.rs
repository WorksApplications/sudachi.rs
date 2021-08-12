use std::path::{Path, PathBuf};

use libloading::{Library, Symbol};

use super::PluginError;
use crate::dic::grammar::Grammar;
use crate::input_text::utf8_input_text_builder::Utf8InputTextBuilder;
use crate::prelude::*;

pub trait InputTextPlugin {
    fn set_up(&mut self, grammar: &Grammar) -> SudachiResult<()>;
    fn rewrite(&self, builder: &mut Utf8InputTextBuilder);
}

/// Declare a plugin type and its constructor.
///
/// # Notes
/// This works by automatically generating an `extern "C"` function with a
/// pre-defined signature and symbol name. Therefore you will only be able to
/// declare one plugin per library.
#[macro_export]
macro_rules! declare_input_text_plugin {
    ($plugin_type:ty, $constructor:path) => {
        #[no_mangle]
        pub extern "C" fn load_plugin() -> *mut InputTextPlugin {
            // make sure the constructor is the correct type.
            let constructor: fn() -> $plugin_type = $constructor;

            let object = constructor();
            let boxed: Box<InputTextPlugin> = Box::new(object);
            Box::into_raw(boxed)
        }
    };
}

#[derive(Default)]
pub struct InputTextPluginManager {
    plugins: Vec<Box<dyn InputTextPlugin>>,
    libraries: Vec<Library>,
}
impl InputTextPluginManager {
    pub fn load(&mut self, path: &Path) -> Result<(), PluginError> {
        type PluginCreate = unsafe fn() -> *mut dyn InputTextPlugin;

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

    pub fn plugins(&self) -> &[Box<dyn InputTextPlugin>] {
        &self.plugins
    }

    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }
}
impl Drop for InputTextPluginManager {
    fn drop(&mut self) {
        // Plugin drop must be called before Library drop.
        self.plugins.clear();
        self.libraries.clear();
    }
}

pub fn get_input_text_plugins(grammar: &Grammar) -> SudachiResult<InputTextPluginManager> {
    // todo load from config
    let mut manager = InputTextPluginManager::default();

    manager.load(&PathBuf::from("./target/debug/libdefault_input_text.so"))?;
    manager.load(&PathBuf::from("./target/debug/libprolonged_sound_mark.so"))?;
    manager.load(&PathBuf::from("./target/debug/libignore_yomigana.so"))?;

    manager.set_up(grammar)?;
    Ok(manager)
}
