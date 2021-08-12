use std::path::{Path, PathBuf};

use libloading::{Library, Symbol};

use super::PluginError;
use crate::dic::grammar::Grammar;
use crate::input_text::utf8_input_text::Utf8InputText;
use crate::lattice::node::Node;
use crate::prelude::*;

pub trait OovProviderPlugin {
    fn set_up(&mut self, grammar: &Grammar) -> SudachiResult<()>;
    fn provide_oov(
        &self,
        input_text: &Utf8InputText,
        offset: usize,
        has_other_words: bool,
    ) -> SudachiResult<Vec<Node>>;

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
        pub extern "C" fn load_plugin() -> *mut OovProviderPlugin {
            // make sure the constructor is the correct type.
            let constructor: fn() -> $plugin_type = $constructor;

            let object = constructor();
            let boxed: Box<OovProviderPlugin> = Box::new(object);
            Box::into_raw(boxed)
        }
    };
}

#[derive(Default)]
pub struct OovProviderPluginManager {
    plugins: Vec<Box<dyn OovProviderPlugin>>,
    libraries: Vec<Library>,
}
impl OovProviderPluginManager {
    pub fn load(&mut self, path: &Path) -> Result<(), PluginError> {
        type PluginCreate = unsafe fn() -> *mut dyn OovProviderPlugin;

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

    pub fn plugins(&self) -> &[Box<dyn OovProviderPlugin>] {
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

pub fn get_oov_plugins(grammar: &Grammar) -> SudachiResult<OovProviderPluginManager> {
    // todo load from config
    let mut manager = OovProviderPluginManager::default();

    manager.load(&PathBuf::from("./target/debug/libsimple_oov.so"))?;
    manager.load(&PathBuf::from("./target/debug/libmecab_oov.so"))?;

    manager.set_up(grammar)?;
    Ok(manager)
}
