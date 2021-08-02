use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader};

use crate::dic::category_type::CategoryType;
use crate::dic::character_category::Error as CharacterCategoryError;
use crate::dic::grammar::Grammar;
use crate::dic::lexicon::word_infos::WordInfo;
use crate::lattice::node::Node;
use crate::plugin::oov::OovProviderPlugin;
use crate::prelude::*;
use crate::utf8inputtext::Utf8InputText;

const DEFAULT_CHAR_DEF_FILE_PATH: &str = "./src/resources/char.def";
const DEFAULT_UNK_DEF_FILE_PATH: &str = "./src/resources/unk.def";

pub struct MeCabOovPlugin {
    categories: HashMap<CategoryType, CategoryInfo>,
    oov_list: HashMap<CategoryType, Vec<OOV>>,
}

impl MeCabOovPlugin {
    pub fn new(grammar: &Grammar) -> SudachiResult<MeCabOovPlugin> {
        // todo: load from file
        let categories = MeCabOovPlugin::read_character_property(DEFAULT_CHAR_DEF_FILE_PATH)?;
        let oov_list = MeCabOovPlugin::read_oov(DEFAULT_UNK_DEF_FILE_PATH, &categories, grammar)?;

        Ok(MeCabOovPlugin {
            categories,
            oov_list,
        })
    }

    fn read_character_property(path: &str) -> SudachiResult<HashMap<CategoryType, CategoryInfo>> {
        // todo: mv to dic/char_category
        let mut categories = HashMap::new();

        let reader = BufReader::new(fs::File::open(&path)?);
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

    fn read_oov(
        path: &str,
        categories: &HashMap<CategoryType, CategoryInfo>,
        grammar: &Grammar,
    ) -> SudachiResult<HashMap<CategoryType, Vec<OOV>>> {
        let mut oov_list: HashMap<CategoryType, Vec<OOV>> = HashMap::new();

        let reader = BufReader::new(fs::File::open(&path)?);
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
                pos_id: grammar
                    .get_part_of_speech_id(&cols[4..10])
                    .ok_or(SudachiError::InvalidPartOfSpeech)?,
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

    fn get_oov_node(&self, text: &str, oov: &OOV, length: u8) -> Node {
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
}

impl OovProviderPlugin for MeCabOovPlugin {
    fn provide_oov(
        &self,
        input_text: &Utf8InputText,
        offset: usize,
        has_other_words: bool,
    ) -> SudachiResult<Vec<Node>> {
        let mut nodes = vec![];
        let byte_len = input_text.get_char_category_continuous_length(offset);
        if byte_len == 0 {
            return Ok(nodes);
        }

        for ctype in input_text.get_char_category_types(offset) {
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
                let s = input_text.get_substring(offset, offset + byte_len)?;
                for oov in oovs {
                    nodes.push(self.get_oov_node(&s, oov, byte_len as u8));
                }
                llength -= 1;
            }
            for i in 1..cinfo.length + 1 {
                let sublength = input_text.get_code_points_offset_length(offset, i as usize);
                if sublength > llength {
                    break;
                }
                let s = input_text.get_substring(offset, offset + sublength)?;
                for oov in oovs {
                    nodes.push(self.get_oov_node(&s, oov, sublength as u8));
                }
            }
        }
        Ok(nodes)
    }
}

#[derive(Debug)]
struct CategoryInfo {
    category_type: CategoryType,
    is_invoke: bool,
    is_group: bool,
    length: u32,
}

#[derive(Debug)]
struct OOV {
    left_id: i16,
    right_id: i16,
    cost: i16,
    pos_id: u16,
}
