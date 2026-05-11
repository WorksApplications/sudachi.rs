/*
 *  Copyright (c) 2021-2026 Works Applications Co., Ltd.
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

use crate::config::Config;
use crate::dic::binary_loader::BinaryDictionary;
use crate::dic::character_category::CharacterCategory;
use crate::dic::description::Description;
use crate::dic::grammar::Grammar;
use crate::dic::lexicon::Lexicon;
use crate::dic::lexicon_set::LexiconSet;
use crate::dic::storage::{Storage, SudachiDicData};
use crate::dic::{DescriptionAccess, DictionaryAccess, LexiconAccess, ReferenceIdAccess};
use crate::error::{SudachiError, SudachiResult};
use crate::plugin::input_text::InputTextPlugin;
use crate::plugin::oov::OovProviderPlugin;
use crate::plugin::path_rewrite::PathRewritePlugin;
use crate::plugin::Plugins;

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
    storage: SudachiDicData,
    plugins: Plugins,
    description: Description,
    //'static is a a lie, lifetime is the same with StorageBackend
    _grammar: Grammar<'static>,
    //'static is a a lie, lifetime is the same with StorageBackend
    _lexicon: LexiconSet<'static>,
}

fn map_file(path: &Path) -> SudachiResult<Storage> {
    let file = File::open(path)?;
    let mapping = unsafe { Mmap::map(&file) }?;
    Ok(Storage::File(mapping))
}

fn load_system_dic(cfg: &Config) -> SudachiResult<Storage> {
    let p = cfg.resolved_system_dict()?;
    map_file(&p).map_err(|e| e.with_context(p.as_os_str().to_string_lossy()))
}

impl JapaneseDictionary {
    /// Creates a dictionary from the specified configuration
    /// Dictionaries will be read from disk
    pub fn from_cfg(cfg: &Config) -> SudachiResult<JapaneseDictionary> {
        let mut sb = SudachiDicData::new(load_system_dic(cfg)?);

        for udic in cfg.resolved_user_dicts()? {
            sb.add_user(
                map_file(&udic).map_err(|e| e.with_context(udic.as_os_str().to_string_lossy()))?,
            )
        }

        let chardef_path = cfg.complete_path(&cfg.character_definition_file)?;
        let chardef = CharacterCategory::from_file(chardef_path.as_path())?;

        Self::from_cfg_storage_chardef(cfg, sb, chardef)
    }

    pub fn from_cfg_storage_chardef(
        cfg: &Config,
        storage: SudachiDicData,
        chardef: CharacterCategory,
    ) -> SudachiResult<JapaneseDictionary> {
        let system_binary =
            BinaryDictionary::load_system(unsafe { storage.system_static_slice() })?;
        let description = system_binary.description.clone();

        let mut grammar = Grammar::from_system_binary(system_binary.grammar)?;
        grammar.set_character_category(chardef);

        let lexicon_set =
            LexiconSet::from_system_binary(system_binary.lexicon, grammar.pos_list.len());

        let plugins = { Plugins::load(cfg, &mut grammar)? };
        if plugins.oov.is_empty() {
            return Err(SudachiError::NoOOVPluginProvided);
        }
        for p in plugins.connect_cost.plugins() {
            p.edit(&mut grammar);
        }

        let mut dic = JapaneseDictionary {
            storage,
            plugins,
            description,
            _grammar: grammar,
            _lexicon: lexicon_set,
        };

        // this Vec is needed to prevent double borrowing of dic
        let user_dicts: Vec<_> = dic.storage.user_static_slice();
        for udic in user_dicts {
            dic = dic.merge_user_dictionary(udic)?;
        }

        Ok(dic)
    }

    /// Creates a dictionary from the specified configuration and storage
    pub fn from_cfg_storage(
        cfg: &Config,
        storage: SudachiDicData,
    ) -> SudachiResult<JapaneseDictionary> {
        let chardef_path = cfg.complete_path(&cfg.character_definition_file)?;
        let chardef = CharacterCategory::from_file(chardef_path.as_path())?;
        Self::from_cfg_storage_chardef(cfg, storage, chardef)
    }

    /// Creates a dictionary from the specified configuration and storage, with embedded character definition
    pub fn from_cfg_storage_with_embedded_chardef(
        cfg: &Config,
        storage: SudachiDicData,
    ) -> SudachiResult<JapaneseDictionary> {
        let chardef = CharacterCategory::from_embedded();
        Self::from_cfg_storage_chardef(cfg, storage, chardef)
    }

    /// Returns grammar with the correct lifetime
    pub fn grammar(&self) -> &Grammar<'_> {
        &self._grammar
    }

    /// Returns lexicon with the correct lifetime
    pub fn lexicon(&self) -> &LexiconSet<'_> {
        &self._lexicon
    }

    pub fn description(&self) -> &Description {
        &self.description
    }

    fn merge_user_dictionary(mut self, dictionary_bytes: &'static [u8]) -> SudachiResult<Self> {
        let user_dict = BinaryDictionary::load_user(dictionary_bytes)?;

        // we need to update lexicon first, since it needs the current number of pos
        let mut user_lexicon = Lexicon::from_binary(user_dict.lexicon);
        user_lexicon.update_cost(&self)?;
        self._lexicon
            .append(user_lexicon, self._grammar.pos_list.len())?;

        self._grammar.merge_binary(user_dict.grammar);

        Ok(self)
    }
}

impl LexiconAccess for JapaneseDictionary {
    fn lexicon(&self) -> &LexiconSet<'_> {
        self.lexicon()
    }
}

impl DictionaryAccess for JapaneseDictionary {
    fn grammar(&self) -> &Grammar<'_> {
        self.grammar()
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

impl DescriptionAccess for JapaneseDictionary {
    fn description(&self) -> &Description {
        &self.description
    }
}

impl ReferenceIdAccess for JapaneseDictionary {
    fn reference_ids(&self) -> std::collections::HashMap<u32, String> {
        BinaryDictionary::load_system(unsafe { self.storage.system_static_slice() })
            .and_then(|dict| dict.reference_id_table())
            .unwrap_or_default()
    }
}
