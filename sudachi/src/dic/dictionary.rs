/*
 *  Copyright (c) 2021 Works Applications Co., Ltd.
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

use std::fs::File;
use std::path::Path;

use memmap2::Mmap;

use crate::analysis::stateless_tokenizer::DictionaryAccess;
use crate::config::Config;
use crate::config::ConfigError::MissingArgument;
use crate::dic::grammar::Grammar;
use crate::dic::lexicon_set::LexiconSet;
use crate::dic::{DictionaryLoader, LoadedDictionary};
use crate::error::SudachiError::ConfigError;
use crate::error::{SudachiError, SudachiResult};
use crate::plugin::input_text::InputTextPlugin;
use crate::plugin::oov::OovProviderPlugin;
use crate::plugin::path_rewrite::PathRewritePlugin;
use crate::plugin::Plugins;

// Mmap stores file handle, so file field may not be needed
// but let it be here for the time being
struct FileMapping {
    #[allow(unused)]
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
        #[allow(unused)] // This is for ownership, this is not used by code
        system: FileMapping,
        user: Vec<FileMapping>,
    },
    // Remove when fixing https://github.com/WorksApplications/sudachi.rs/issues/35
    #[allow(unused)]
    InMemory {},
}

impl StorageBackend {
    pub(crate) fn user_dicts_iter(&self) -> impl Iterator<Item = &'static [u8]> + '_ {
        match self {
            StorageBackend::FileSystem { user: u, .. } => {
                u.iter().map(|m| unsafe { m.as_static_slice() })
            }
            _ => panic!("Not implemented"),
        }
    }
}

impl Drop for StorageBackend {
    fn drop(&mut self) {
        match self {
            StorageBackend::FileSystem { user: u, .. } => {
                u.clear();
            }
            _ => panic!("Not implemented"),
        }
    }
}

// It is self-referential struct with 'static lifetime as a workaround
// for the impossibility to specify the correct lifetime for
// those fields. Accessor functions always provide the correct lifetime,
// tied to the lifetime of the struct itself.
// It is safe to move this structure around because the
// pointers from memory mappings themselves are stable and
// will not change if the structure will be moved around.
// This structure is always read only after creation and is safe to share
// between threads.
pub struct JapaneseDictionary {
    backend: StorageBackend,
    plugins: Plugins,
    //'static is a a lie, lifetime is the same with StorageBackend
    _grammar: Grammar<'static>,
    //'static is a a lie, lifetime is the same with StorageBackend
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
    pub fn from_cfg(cfg: &Config) -> SudachiResult<JapaneseDictionary> {
        let sys_dic = load_system_dic(cfg)?;
        let user_dics = load_user_dics(cfg)?;

        let mut basic_dict = LoadedDictionary::from_system_dictionary(
            unsafe { sys_dic.as_static_slice() },
            &cfg.character_definition_file,
        )?;

        let plugins = Plugins::load(cfg, &basic_dict.grammar)?;

        if plugins.oov.is_empty() {
            return Err(SudachiError::NoOOVPluginProvided);
        }

        for p in plugins.connect_cost.plugins() {
            p.edit(&mut basic_dict.grammar);
        }

        let mut dic = JapaneseDictionary {
            backend: StorageBackend::FileSystem {
                system: sys_dic,
                user: user_dics,
            },
            plugins: plugins,
            _grammar: basic_dict.grammar,
            _lexicon: basic_dict.lexicon_set,
        };

        // this Vec is needed to prevent double borrowing of dic
        let user_dicts: Vec<_> = dic.backend.user_dicts_iter().collect();
        for udic in user_dicts {
            dic = dic.merge_user_dictionary(udic)?;
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

    fn merge_user_dictionary(mut self, dictionary_bytes: &'static [u8]) -> SudachiResult<Self> {
        let user_dict = DictionaryLoader::read_user_dictionary(dictionary_bytes)?;

        // we need to update lexicon first, since it needs the current number of pos
        let mut user_lexicon = user_dict.lexicon;
        user_lexicon.update_cost(&self)?;

        self._lexicon
            .append(user_lexicon, self._grammar.pos_list.len())?;

        if let Some(g) = user_dict.grammar {
            self._grammar.merge(g);
        }

        Ok(self)
    }
}

impl DictionaryAccess for JapaneseDictionary {
    fn grammar(&self) -> &Grammar<'_> {
        self.grammar()
    }

    fn lexicon(&self) -> &LexiconSet<'_> {
        self.lexicon()
    }

    fn input_text_plugins(&self) -> &[Box<dyn InputTextPlugin + Sync + Send>] {
        self.plugins.input_text.plugins()
    }

    fn oov_provider_plugins(&self) -> &[Box<dyn OovProviderPlugin + Sync + Send>] {
        self.plugins.oov.plugins()
    }

    fn path_rewrite_plugins(&self) -> &[Box<dyn PathRewritePlugin + Sync + Send>] {
        self.plugins.path_rewrite.plugins()
    }
}
