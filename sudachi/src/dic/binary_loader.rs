/*
 * Copyright (c) 2026 Works Applications Co., Ltd.
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

use crate::dic::connect::ConnectionMatrix;
use crate::dic::description::{Block, Description};
use crate::dic::grammar::Grammar;
use crate::dic::header::HeaderError;
use crate::dic::lexicon::strings::CompactedStrings;
use crate::dic::lexicon::trie::Trie;
use crate::dic::lexicon::word_id_table::WordIdTable;
use crate::dic::lexicon::word_params::WordParams;
use crate::dic::lexicon::Lexicon;
use crate::dic::lexicon_set::LexiconSet;
use crate::dic::pos::PosList;
use crate::dic::read::utf8_string::utf8_string;
use crate::dic::read::varint::varint32;
use crate::dic::word_info::WordInfos;
use crate::dic::{DescriptionAccess, DictionaryAccess, LexiconAccess, ReferenceIdAccess};
use crate::plugin::input_text::InputTextPlugin;
use crate::plugin::oov::OovProviderPlugin;
use crate::plugin::path_rewrite::PathRewritePlugin;
use crate::prelude::*;
use std::collections::HashMap;

/// A single system or user dictionary
pub struct BinaryDictionary<'a> {
    raw_bytes: &'a [u8],
    pub description: Description,
    pub grammar: BinaryGrammar<'a>,
    pub lexicon: BinaryLexicon<'a>,
}

impl<'a> BinaryDictionary<'a> {
    /// Load a binary dictionary from bytes
    ///
    /// # Safety
    /// This function is marked unsafe because it does not perform header validation
    unsafe fn load(buf: &'a [u8]) -> SudachiResult<Self> {
        let description = Description::load(buf)?;
        let grammar = BinaryGrammar::load(buf, &description)?;
        let lexicon = BinaryLexicon::load(buf, &description)?;

        Ok(BinaryDictionary {
            raw_bytes: buf,
            description,
            grammar,
            lexicon,
        })
    }

    pub fn load_system(buf: &'a [u8]) -> SudachiResult<Self> {
        let dict = unsafe { Self::load(buf)? };

        if dict.description.is_system_dictionary() {
            Ok(dict)
        } else {
            // TODO: fix error type
            Err(SudachiError::InvalidHeader(
                HeaderError::InvalidSystemDictVersion,
            ))
        }
    }

    pub fn load_user(buf: &'a [u8]) -> SudachiResult<Self> {
        let dict = unsafe { Self::load(buf)? };

        if dict.description.is_user_dictionary() {
            Ok(dict)
        } else {
            // TODO: fix error type
            Err(SudachiError::InvalidHeader(
                HeaderError::InvalidUserDictVersion,
            ))
        }
    }

    /// Build-time helper for dictionary builders and dump tooling.
    /// Runtime tokenization does not use this table.
    pub fn reference_id_table(&self) -> SudachiResult<HashMap<u32, String>> {
        let Some(bytes) = self
            .description
            .slice_or_none(self.raw_bytes, Block::ReferenceIdTable)?
        else {
            return Ok(HashMap::with_capacity(0));
        };
        parse_reference_id_table(bytes)
    }
}

/// Grammar part of the single binary dictionary
pub struct BinaryGrammar<'a> {
    /// The list of part of speechs
    pub pos_list: PosList,

    /// The overloadable connection cost matrix
    ///
    /// Only system dictionary has this.
    pub connection: Option<ConnectionMatrix<'a>>,
}

impl<'a> BinaryGrammar<'a> {
    /// load a grammar from bytes
    pub fn load(buf: &'a [u8], description: &Description) -> SudachiResult<Self> {
        let connection_bytes = description.slice_or_none(buf, Block::ConnectionMatrix)?;
        let connection = match connection_bytes {
            Some(bytes) => {
                let connection = ConnectionMatrix::from_bytes(bytes)?;
                Some(connection)
            }
            None => None,
        };

        let pos_list = PosList::from_bytes(description.slice(buf, Block::POSTable)?)?;

        Ok(Self {
            pos_list,
            connection,
        })
    }
}

/// Lexicon part of the single binary dictionary
pub struct BinaryLexicon<'a> {
    /// TRIE (double array), mapping from index form to WordIdTable offset
    pub trie: Trie<'a>,
    /// list of word ids that have the same index form
    pub word_id_table: WordIdTable<'a>,
    /// list of word information (for analysis)
    pub word_params: WordParams<'a>,
    /// list of word information (for non-analysis)
    pub word_infos: WordInfos<'a>,
    /// Stotage of strings in the lixicon (normalized form etc.)
    pub strings: CompactedStrings<'a>,
    /// The number of entries in the lexicon
    pub num_total_entries: u32,
}

impl<'a> BinaryLexicon<'a> {
    /// load a lexicon from bytes
    pub fn load(buf: &'a [u8], description: &Description) -> SudachiResult<Self> {
        let trie = Trie::from_bytes(description.slice(buf, Block::TRIEIndex)?);
        let word_id_table = WordIdTable::from_bytes(description.slice(buf, Block::WordPointers)?);

        // word_params and word_infos share the same byte range.
        // the first 8 bytes of a word entry is the paramaters and rest is the infos.
        // handle separately because we use them in different steps; during/after analysis.
        let entries_bytes = description.slice(buf, Block::Entries)?;
        let word_params = WordParams::from_bytes(entries_bytes);
        let word_infos = WordInfos::from_bytes(entries_bytes);

        let strings = CompactedStrings::from_bytes(description.slice(buf, Block::Strings)?);

        Ok(Self {
            trie,
            word_id_table,
            word_params,
            word_infos,
            strings,
            num_total_entries: description.num_total_entries(),
        })
    }
}

/// A dictionary consists of one system_dict and zero or more user_dicts.
///
/// This is mostly used for testing purpose.
pub struct LoadedDictionary<'a> {
    pub description: Description,
    pub grammar: Grammar<'a>,
    pub lexicon_set: LexiconSet<'a>,
    system_reference_ids: HashMap<u32, String>,
}

impl<'a> LoadedDictionary<'a> {
    /// Convert to Loaded dictionary
    pub fn from_system_binary(binary: BinaryDictionary<'a>) -> SudachiResult<Self> {
        let system_reference_ids = binary.reference_id_table()?;
        let description = binary.description;
        let grammar = Grammar::from_system_binary(binary.grammar)?;
        let lexicon_set = LexiconSet::from_system_binary(binary.lexicon, grammar.pos_list.len());
        Ok(LoadedDictionary {
            description,
            grammar,
            lexicon_set,
            system_reference_ids,
        })
    }

    pub fn load_system(bytes: &'a [u8]) -> SudachiResult<Self> {
        Self::from_system_binary(BinaryDictionary::load_system(bytes)?)
    }

    pub fn description(&self) -> &Description {
        &self.description
    }

    pub fn merge_dictionary(mut self, other: BinaryDictionary<'a>) -> SudachiResult<Self> {
        self.lexicon_set.append(
            Lexicon::from_binary(other.lexicon),
            self.grammar.pos_list.len(),
        )?;
        self.grammar.merge_binary(other.grammar);
        Ok(self)
    }
}

impl LexiconAccess for LoadedDictionary<'_> {
    fn lexicon(&self) -> &LexiconSet<'_> {
        &self.lexicon_set
    }
}

impl DictionaryAccess for LoadedDictionary<'_> {
    fn grammar(&self) -> &Grammar<'_> {
        &self.grammar
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

impl DescriptionAccess for LoadedDictionary<'_> {
    fn description(&self) -> &Description {
        &self.description
    }
}

impl ReferenceIdAccess for BinaryDictionary<'_> {
    fn reference_ids(&self) -> HashMap<u32, String> {
        self.reference_id_table().unwrap_or_default()
    }
}

impl ReferenceIdAccess for LoadedDictionary<'_> {
    fn reference_ids(&self) -> HashMap<u32, String> {
        self.system_reference_ids.clone()
    }
}

fn parse_reference_id_table(bytes: &[u8]) -> SudachiResult<HashMap<u32, String>> {
    let (mut rest, count) = varint32(bytes)?;
    let mut result = HashMap::with_capacity(count as usize);
    for _ in 0..count {
        let (new_rest, entry_id) = varint32(rest)?;
        let (new_rest, reference_id) = utf8_string(new_rest)?;
        result.insert(entry_id, reference_id);
        rest = new_rest;
    }
    Ok(result)
}
