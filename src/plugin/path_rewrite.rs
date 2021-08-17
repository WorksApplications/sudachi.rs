use std::path::Path;

use libloading::{Library, Symbol};
use serde_json::Value;

use crate::config::Config;
use crate::dic::grammar::Grammar;
use crate::dic::lexicon::word_infos::WordInfo;
use crate::input_text::utf8_input_text::Utf8InputText;
use crate::lattice::{node::Node, Lattice};
use crate::prelude::*;

pub trait PathRewritePlugin {
    fn set_up(&mut self, settings: &Value, config: &Config, grammar: &Grammar)
        -> SudachiResult<()>;
    fn rewrite(
        &self,
        text: &Utf8InputText,
        path: Vec<Node>,
        lattice: &Lattice,
    ) -> SudachiResult<Vec<Node>>;

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
            .map(|node| node.word_info.clone())
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

        let mut node = Node::new_default();
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
            .map(|node| node.word_info.clone())
            .collect::<Option<_>>()
            .ok_or(SudachiError::MissingWordInfo)?;
        let surface = word_infos
            .iter()
            .fold(String::new(), |acc, wi| acc + &wi.surface);
        let head_word_length = word_infos
            .iter()
            .fold(0, |acc, wi| acc + wi.head_word_length);

        let mut node = Node::new_default();
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

pub fn get_path_rewrite_plugins(
    config: &Config,
    grammar: &Grammar,
) -> SudachiResult<PathRewritePluginManager> {
    let mut manager = PathRewritePluginManager::default();

    for plugin in &config.path_rewrite_plugins {
        let lib = super::get_plugin_path(plugin)?;
        manager.load(lib, plugin, config, grammar)?;
    }

    Ok(manager)
}
