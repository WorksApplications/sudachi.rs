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

use crate::analysis::Node;
use crate::config::Config;
use crate::dic::category_type::CategoryType;
use crate::dic::character_category::Error as CharacterCategoryError;
use crate::dic::grammar::Grammar;
use crate::dic::word_id::WordId;
use crate::hash::RoMu;
use crate::input_text::InputBuffer;
use crate::input_text::InputTextIndex;
use crate::plugin::oov::OovProviderPlugin;
use crate::prelude::*;

#[cfg(test)]
mod tests;

const DEFAULT_CHAR_DEF_FILE: &str = "char.def";
const DEFAULT_UNK_DEF_FILE: &str = "unk.def";

/// provides MeCab oov nodes
#[derive(Default)]
pub struct MeCabOovPlugin {
    categories: HashMap<CategoryType, CategoryInfo, RoMu>,
    oov_list: HashMap<CategoryType, Vec<OOV>, RoMu>,
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
    ) -> SudachiResult<HashMap<CategoryType, CategoryInfo, RoMu>> {
        let mut categories = HashMap::with_hasher(RoMu::new());
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
        categories: &HashMap<CategoryType, CategoryInfo, RoMu>,
        grammar: &Grammar,
    ) -> SudachiResult<HashMap<CategoryType, Vec<OOV>, RoMu>> {
        let mut oov_list: HashMap<CategoryType, Vec<OOV>, RoMu> = HashMap::with_hasher(RoMu::new());
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

            if oov.left_id as usize > grammar.conn_matrix().num_left() {
                return Err(SudachiError::InvalidDataFormat(
                    0,
                    format!(
                        "max grammar left_id is {}, was {}",
                        grammar.conn_matrix().num_left(),
                        oov.left_id
                    ),
                ));
            }

            if oov.right_id as usize > grammar.conn_matrix().num_right() {
                return Err(SudachiError::InvalidDataFormat(
                    0,
                    format!(
                        "max grammar right_id is {}, was {}",
                        grammar.conn_matrix().num_right(),
                        oov.right_id
                    ),
                ));
            }

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
    fn get_oov_node(&self, oov: &OOV, start: usize, end: usize) -> Node {
        Node::new(
            start as u16,
            end as u16,
            oov.left_id as u16,
            oov.right_id as u16,
            oov.cost,
            WordId::oov(oov.pos_id as u32),
        )
    }

    fn provide_oov_gen<T: InputTextIndex>(
        &self,
        input: &T,
        offset: usize,
        has_other_words: bool,
        nodes: &mut Vec<Node>,
    ) -> SudachiResult<()> {
        let char_len = input.cat_continuous_len(offset);
        if char_len == 0 {
            return Ok(());
        }

        for ctype in input.cat_at_char(offset).iter() {
            let cinfo = match self.categories.get(&ctype) {
                Some(ci) => ci,
                None => continue,
            };
            if !cinfo.is_invoke && has_other_words {
                continue;
            }

            let mut llength = char_len;
            let oovs = match self.oov_list.get(&cinfo.category_type) {
                Some(v) => v,
                None => continue,
            };

            if cinfo.is_group {
                for oov in oovs {
                    nodes.push(self.get_oov_node(oov, offset, offset + char_len));
                }
                llength -= 1;
            }
            for i in 1..=cinfo.length {
                let sublength = input.char_distance(offset, i as usize);
                if sublength > llength {
                    break;
                }
                for oov in oovs {
                    nodes.push(self.get_oov_node(oov, offset, offset + sublength));
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
