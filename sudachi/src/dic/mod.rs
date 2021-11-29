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

use std::path::PathBuf;

use crate::analysis::stateless_tokenizer::DictionaryAccess;
use character_category::CharacterCategory;
use grammar::Grammar;
use header::Header;
use lexicon::Lexicon;
use lexicon_set::LexiconSet;

use crate::plugin::input_text::InputTextPlugin;
use crate::plugin::oov::OovProviderPlugin;
use crate::plugin::path_rewrite::PathRewritePlugin;
use crate::prelude::*;

pub mod build;
pub mod category_type;
pub mod character_category;
pub mod connect;
pub mod dictionary;
pub mod grammar;
pub mod header;
pub mod lexicon;
pub mod lexicon_set;
pub mod read;
pub mod storage;
pub mod subset;
pub mod word_id;

const POS_DEPTH: usize = 6;

/// A dictionary consists of one system_dict and zero or more user_dicts
pub struct LoadedDictionary<'a> {
    pub grammar: Grammar<'a>,
    pub lexicon_set: LexiconSet<'a>,
}

impl<'a> LoadedDictionary<'a> {
    /// Creates a system dictionary from bytes, and load a character category from file
    pub fn from_system_dictionary(
        dictionary_bytes: &'a [u8],
        character_category_file: &PathBuf,
    ) -> SudachiResult<LoadedDictionary<'a>> {
        let system_dict = DictionaryLoader::read_system_dictionary(dictionary_bytes)?;

        let character_category = CharacterCategory::from_file(character_category_file)?;
        let mut grammar = system_dict
            .grammar
            .ok_or(SudachiError::InvalidDictionaryGrammar)?;
        grammar.set_character_category(character_category);

        Ok(LoadedDictionary {
            grammar,
            lexicon_set: LexiconSet::new(system_dict.lexicon),
        })
    }

    #[cfg(test)]
    pub(crate) fn merge_dictionary(
        mut self,
        other: DictionaryLoader<'a>,
    ) -> SudachiResult<LoadedDictionary> {
        let npos = self.grammar.pos_list.len();
        let lexicon = other.lexicon;
        let grammar = other.grammar;
        self.lexicon_set.append(lexicon, npos)?;
        grammar.map(|g| self.grammar.merge(g));
        Ok(self)
    }
}

impl<'a> DictionaryAccess for LoadedDictionary<'a> {
    fn grammar(&self) -> &Grammar<'a> {
        &self.grammar
    }

    fn lexicon(&self) -> &LexiconSet<'a> {
        &self.lexicon_set
    }

    fn input_text_plugins(&self) -> &[Box<dyn InputTextPlugin + Sync + Send>] {
        &[]
    }

    fn oov_provider_plugins(&self) -> &[Box<dyn OovProviderPlugin + Sync + Send>] {
        &[]
    }

    fn path_rewrite_plugins(&self) -> &[Box<dyn PathRewritePlugin + Sync + Send>] {
        &[]
    }
}

/// A single system or user dictionary
pub struct DictionaryLoader<'a> {
    pub header: Header,
    pub grammar: Option<Grammar<'a>>,
    pub lexicon: Lexicon<'a>,
}

impl<'a> DictionaryLoader<'a> {
    /// Creates a binary dictionary from bytes
    ///
    /// This function is marked unsafe because it does not perform header validation
    pub unsafe fn read_any_dictionary(dictionary_bytes: &[u8]) -> SudachiResult<DictionaryLoader> {
        let header = Header::parse(&dictionary_bytes[..Header::STORAGE_SIZE])?;
        let mut offset = Header::STORAGE_SIZE;

        let grammar = if header.has_grammar() {
            let tmp = Grammar::parse(dictionary_bytes, offset)?;
            offset += tmp.storage_size;
            Some(tmp)
        } else {
            None
        };

        let lexicon = Lexicon::parse(dictionary_bytes, offset, header.has_synonym_group_ids())?;

        Ok(DictionaryLoader {
            header,
            grammar,
            lexicon,
        })
    }

    /// Creates a system binary dictionary from bytes
    ///
    /// Returns Err if header version is not match
    pub fn read_system_dictionary(dictionary_bytes: &[u8]) -> SudachiResult<DictionaryLoader> {
        let dict = unsafe { Self::read_any_dictionary(dictionary_bytes) }?;
        match dict.header.version {
            header::HeaderVersion::SystemDict(_) => Ok(dict),
            _ => Err(SudachiError::InvalidHeader(
                header::HeaderError::InvalidSystemDictVersion,
            )),
        }
    }

    /// Creates a user binary dictionary from bytes
    ///
    /// Returns Err if header version is not match
    pub fn read_user_dictionary(dictionary_bytes: &[u8]) -> SudachiResult<DictionaryLoader> {
        let dict = unsafe { Self::read_any_dictionary(dictionary_bytes) }?;
        match dict.header.version {
            header::HeaderVersion::UserDict(_) => Ok(dict),
            _ => Err(SudachiError::InvalidHeader(
                header::HeaderError::InvalidSystemDictVersion,
            )),
        }
    }

    pub fn to_loaded(self) -> Option<LoadedDictionary<'a>> {
        let mut lexicon = self.lexicon;
        lexicon.set_dic_id(0);
        match self.grammar {
            None => None,
            Some(grammar) => Some(LoadedDictionary {
                grammar,
                lexicon_set: LexiconSet::new(lexicon),
            }),
        }
    }
}
