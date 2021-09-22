use crate::config::Config;
use crate::config::ConfigError::MissingArgument;
use crate::dic::grammar::Grammar;
use crate::dic::lexicon_set::LexiconSet;
use crate::dic::{DictionaryLoader, LoadedDictionary};
use crate::error::SudachiError::ConfigError;
use crate::error::SudachiResult;
use crate::plugin::input_text::InputTextPlugin;
use crate::plugin::oov::OovProviderPlugin;
use crate::plugin::path_rewrite::PathRewritePlugin;
use crate::plugin::{Plugins, PluginProvider};
use crate::stateless_tokeniser::{DictionaryAccess, StatelessTokenizer};
use memmap2::Mmap;
use std::fs::File;
use std::path::Path;

struct FileMapping {
    file: File,
    mapping: Mmap,
}

impl FileMapping {
    // super unsafe, but we don't leak static lifetime outside of dictionary
    // this is used only for construction of dictionary parts
    unsafe fn as_static_slice(&self) -> &'static [u8] {
        let slice: &[u8] = self.mapping.as_ref();
        std::mem::transmute(slice)
    }
}

enum StorageBackend {
    FileSystem {
        system: FileMapping,
        user: Vec<FileMapping>,
    },
    InMemory {},
}

impl StorageBackend {
    pub(crate) fn sys_dict(&self) -> &'static [u8] {
        match self {
            StorageBackend::FileSystem { system: s, .. } => unsafe { s.as_static_slice() },
            _ => todo!(),
        }
    }

    pub(crate) fn user_dicts_iter(&self) -> impl Iterator<Item = &'static [u8]> + '_ {
        match self {
            StorageBackend::FileSystem { user: u, .. } => {
                u.iter().map(|m| unsafe { m.as_static_slice() })
            }
            _ => todo!(),
        }
    }
}

impl Drop for StorageBackend {
    fn drop(&mut self) {
        match self {
            StorageBackend::FileSystem { system: s, user: u } => {
                u.clear();
            }
            _ => todo!(),
        }
    }
}

pub struct JapaneseDictionary {
    backend: StorageBackend,
    plugins: Plugins,
    _grammar: Grammar<'static>,
    _lexicon: LexiconSet<'static>,
}

fn map_file(path: &Path) -> SudachiResult<FileMapping> {
    let file = File::open(path)?;
    let mapping = unsafe { Mmap::map(&file) }?;
    Ok(FileMapping { file, mapping })
}

fn load_system_dic(cfg: &Config) -> SudachiResult<FileMapping> {
    match &cfg.system_dict {
        Some(p) => map_file(p),
        None => return Err(ConfigError(MissingArgument(String::from("system_dict")))),
    }
}

fn load_user_dics(cfg: &Config) -> SudachiResult<Vec<FileMapping>> {
    cfg.user_dicts.iter().map(|p| map_file(p)).collect()
}

impl JapaneseDictionary {
    pub(crate) fn from_cfg(cfg: &Config) -> SudachiResult<JapaneseDictionary> {
        let sys_dic = load_system_dic(cfg)?;
        let user_dics = load_user_dics(cfg)?;

        let basic_dict = LoadedDictionary::from_system_dictionary(
            unsafe { sys_dic.as_static_slice() },
            &cfg.character_definition_file,
        )?;

        let mut dic = JapaneseDictionary {
            backend: StorageBackend::FileSystem {
                system: sys_dic,
                user: user_dics,
            },
            plugins: Plugins::new(),
            _grammar: basic_dict.grammar,
            _lexicon: basic_dict.lexicon_set,
        };

        // this Vec is needed to prevent double borrowing of dic
        let user_dicts: Vec<_> = dic.backend.user_dicts_iter().collect();
        for udic in user_dicts {
            dic.merge_user_dictionary(udic)?;
        }

        Ok(dic)
    }

    /// Returns grammar with the correct lifetime
    pub fn grammar<'a>(&'a self) -> &Grammar<'a> {
        &self._grammar
    }

    /// Returns lexicon with the correct lifetime
    pub fn lexicon<'a>(&'a self) -> &LexiconSet<'a> {
        &self._lexicon
    }

    fn merge_user_dictionary(&mut self, dictionary_bytes: &'static [u8]) -> SudachiResult<()> {
        let user_dict = DictionaryLoader::read_user_dictionary(dictionary_bytes)?;

        // we need to update lexicon first, since it needs the current number of pos
        let mut user_lexicon = user_dict.lexicon;
        let tok = StatelessTokenizer::new(self);

        user_lexicon.update_cost(&tok)?;
        self._lexicon
            .append(user_lexicon, self._grammar.pos_list.len())?;

        if let Some(g) = user_dict.grammar {
            self._grammar.merge(g);
        }

        Ok(())
    }
}

impl DictionaryAccess for JapaneseDictionary {
    fn grammar(&self) -> &Grammar<'_> {
        self.grammar()
    }

    fn lexicon(&self) -> &LexiconSet<'_> {
        self.lexicon()
    }

    fn input_text_plugins(&self) -> &[Box<dyn InputTextPlugin>] {
        self.plugins.input_text.plugins()
    }

    fn oov_provider_plugins(&self) -> &[Box<dyn OovProviderPlugin>] {
        todo!()
    }

    fn path_rewrite_plugins(&self) -> &[Box<dyn PathRewritePlugin>] {
        todo!()
    }
}
