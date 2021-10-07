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

use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use crate::analysis::node::Node;
use crate::config::Config;
use crate::dic::category_type::CategoryType;
use crate::dic::character_category::Error as CharacterCategoryError;
use crate::dic::grammar::Grammar;
use crate::dic::lexicon::word_infos::WordInfo;
use crate::input_text::input_buffer::InputBuffer;
use crate::input_text::{InputTextIndex, Utf8InputText};
use crate::plugin::oov::OovProviderPlugin;
use crate::prelude::*;

#[cfg(test)]
mod tests;

const DEFAULT_CHAR_DEF_FILE: &str = "char.def";
const DEFAULT_UNK_DEF_FILE: &str = "unk.def";

/// provides MeCab oov nodes
#[derive(Default)]
pub struct MeCabOovPlugin {
    categories: HashMap<CategoryType, CategoryInfo>,
    oov_list: HashMap<CategoryType, Vec<OOV>>,
}

/// Struct corresponds with raw config json file.
#[allow(non_snake_case)]
#[derive(Deserialize)]
struct PluginSettings {
    charDef: Option<PathBuf>,
    unkDef: Option<PathBuf>,
}

impl MeCabOovPlugin {
    /// Loads character category definition
    ///
    /// See resources/char.def for the syntax
    fn read_character_property<T: BufRead>(
        reader: T,
    ) -> SudachiResult<HashMap<CategoryType, CategoryInfo>> {
        let mut categories = HashMap::new();
        for (i, line) in reader.lines().enumerate() {
            let line = line?;
            let line = line.trim();
            if line.is_empty()
                || line.chars().next().unwrap() == '#'
                || line.chars().take(2).collect::<Vec<_>>() == vec!['0', 'x']
            {
                continue;
            }

            let cols: Vec<_> = line.split_whitespace().collect();
            if cols.len() < 4 {
                return Err(SudachiError::InvalidCharacterCategory(
                    CharacterCategoryError::InvalidFormat(i),
                ));
            }
            let category_type: CategoryType = match cols[0].parse() {
                Ok(t) => t,
                Err(_) => {
                    return Err(SudachiError::InvalidCharacterCategory(
                        CharacterCategoryError::InvalidCategoryType(i, cols[0].to_string()),
                    ))
                }
            };
            if categories.contains_key(&category_type) {
                return Err(SudachiError::InvalidCharacterCategory(
                    CharacterCategoryError::MultipleTypeDefinition(i, cols[0].to_string()),
                ));
            }

            categories.insert(
                category_type,
                CategoryInfo {
                    category_type,
                    is_invoke: cols[1] == "1",
                    is_group: cols[2] == "1",
                    length: cols[3].parse()?,
                },
            );
        }

        Ok(categories)
    }

    /// Load OOV definition
    ///
    /// Each line contains: CategoryType, left_id, right_id, cost, and pos
    fn read_oov<T: BufRead>(
        reader: T,
        categories: &HashMap<CategoryType, CategoryInfo>,
        grammar: &Grammar,
    ) -> SudachiResult<HashMap<CategoryType, Vec<OOV>>> {
        let mut oov_list: HashMap<CategoryType, Vec<OOV>> = HashMap::new();
        for (i, line) in reader.lines().enumerate() {
            let line = line?;
            let line = line.trim();
            if line.is_empty() || line.chars().next().unwrap() == '#' {
                continue;
            }

            let cols: Vec<_> = line.split(',').collect();
            if cols.len() < 10 {
                return Err(SudachiError::InvalidDataFormat(i, format!("{}", line)));
            }
            let category_type: CategoryType = cols[0].parse()?;
            if !categories.contains_key(&category_type) {
                return Err(SudachiError::InvalidDataFormat(
                    i,
                    format!("{} is undefined in char definition", cols[0]),
                ));
            }

            let oov = OOV {
                left_id: cols[1].parse()?,
                right_id: cols[2].parse()?,
                cost: cols[3].parse()?,
                pos_id: grammar.get_part_of_speech_id(&cols[4..10]).ok_or(
                    SudachiError::InvalidPartOfSpeech(format!("{:?}", &cols[4..10])),
                )?,
            };
            match oov_list.get_mut(&category_type) {
                None => {
                    oov_list.insert(category_type, vec![oov]);
                }
                Some(l) => {
                    l.push(oov);
                }
            };
        }

        Ok(oov_list)
    }

    /// Creates a new oov node
    fn get_oov_node(&self, text: &str, oov: &OOV, length: u16) -> Node {
        let surface = String::from(text);
        let word_info = WordInfo {
            normalized_form: surface.clone(),
            dictionary_form: surface.clone(),
            surface,
            head_word_length: length,
            pos_id: oov.pos_id,
            dictionary_form_word_id: -1,
            ..Default::default()
        };
        Node::new_oov(oov.left_id, oov.right_id, oov.cost, word_info)
    }

    fn provide_oov_gen<T: InputTextIndex>(
        &self,
        input: &T,
        offset: usize,
        has_other_words: bool,
        mut nodes: &mut Vec<Node>,
    ) -> SudachiResult<()> {
        let byte_len = input.cat_continuous_len(offset);
        if byte_len == 0 {
            return Ok(());
        }

        for ctype in input.cat_at_byte(offset).iter() {
            let cinfo = match self.categories.get(&ctype) {
                Some(ci) => ci,
                None => continue,
            };
            if !cinfo.is_invoke && has_other_words {
                continue;
            }

            let mut llength = byte_len;
            let oovs = match self.oov_list.get(&cinfo.category_type) {
                Some(v) => v,
                None => continue,
            };

            if cinfo.is_group {
                let s = input.curr_slice(offset..offset + byte_len);
                for oov in oovs {
                    nodes.push(self.get_oov_node(&s, oov, byte_len as u16));
                }
                llength -= 1;
            }
            for i in 1..cinfo.length + 1 {
                let sublength = input.byte_distance(offset, i as usize);
                if sublength > llength {
                    break;
                }
                let s = input.curr_slice(offset..offset + sublength);
                for oov in oovs {
                    nodes.push(self.get_oov_node(&s, oov, sublength as u16));
                }
            }
        }
        Ok(())
    }
}

impl OovProviderPlugin for MeCabOovPlugin {
    fn set_up(
        &mut self,
        settings: &Value,
        config: &Config,
        grammar: &Grammar,
    ) -> SudachiResult<()> {
        let settings: PluginSettings = serde_json::from_value(settings.clone())?;

        let char_def_path = config.complete_path(
            settings
                .charDef
                .unwrap_or_else(|| PathBuf::from(DEFAULT_CHAR_DEF_FILE)),
        );
        let reader = BufReader::new(fs::File::open(&char_def_path)?);
        let categories = MeCabOovPlugin::read_character_property(reader)?;

        let unk_def_path = config.complete_path(
            settings
                .unkDef
                .unwrap_or_else(|| PathBuf::from(DEFAULT_UNK_DEF_FILE)),
        );
        let reader = BufReader::new(fs::File::open(&unk_def_path)?);
        let oov_list = MeCabOovPlugin::read_oov(reader, &categories, grammar)?;

        self.categories = categories;
        self.oov_list = oov_list;

        Ok(())
    }

    fn provide_oov(
        &self,
        input_text: &Utf8InputText,
        offset: usize,
        has_other_words: bool,
    ) -> SudachiResult<Vec<Node>> {
        let mut nodes = vec![];
        self.provide_oov_gen(input_text, offset, has_other_words, &mut nodes);
        Ok(nodes)
    }

    fn provide_oov2(
        &self,
        input_text: &InputBuffer,
        offset: usize,
        has_other_words: bool,
        result: &mut Vec<Node>,
    ) -> SudachiResult<()> {
        self.provide_oov_gen(input_text, offset, has_other_words, result)
    }
}

/// The character category definition
#[derive(Debug)]
struct CategoryInfo {
    category_type: CategoryType,
    is_invoke: bool,
    is_group: bool,
    length: u32,
}

/// The OOV definition
#[derive(Debug, Default, Clone)]
struct OOV {
    left_id: i16,
    right_id: i16,
    cost: i16,
    pos_id: u16,
}
