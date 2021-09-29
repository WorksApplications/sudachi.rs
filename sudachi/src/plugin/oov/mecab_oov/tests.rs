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

use std::io::{Seek, SeekFrom, Write};

use claim::assert_matches;
use tempfile::tempfile;

use crate::dic::category_type::CategoryTypes;

use super::*;

#[test]
fn provide_oov000() {
    let mut plugin = build_plugin();
    plugin.categories.insert(
        CategoryType::KANJI,
        CategoryInfo {
            category_type: CategoryType::KANJI,
            is_invoke: false,
            is_group: false,
            length: 0,
        },
    );
    let text = "あいうえお";
    let text = build_input_text(text, 0, 3, CategoryType::KANJI);

    let nodes = plugin
        .provide_oov(&text, 0, false)
        .expect("Failed to generate oovs");
    assert!(nodes.is_empty());

    let nodes = plugin
        .provide_oov(&text, 0, true)
        .expect("Failed to generate oovs");
    assert!(nodes.is_empty());
}

#[test]
fn provide_oov100() {
    let mut plugin = build_plugin();
    plugin.categories.insert(
        CategoryType::KANJI,
        CategoryInfo {
            category_type: CategoryType::KANJI,
            is_invoke: true,
            is_group: false,
            length: 0,
        },
    );
    let text = "あいうえお";
    let text = build_input_text(text, 0, 3, CategoryType::KANJI);

    let nodes = plugin
        .provide_oov(&text, 0, false)
        .expect("Failed to generate oovs");
    assert!(nodes.is_empty());

    let nodes = plugin
        .provide_oov(&text, 0, true)
        .expect("Failed to generate oovs");
    assert!(nodes.is_empty());
}

#[test]
fn provide_oov010() {
    let mut plugin = build_plugin();
    plugin.categories.insert(
        CategoryType::KANJI,
        CategoryInfo {
            category_type: CategoryType::KANJI,
            is_invoke: false,
            is_group: true,
            length: 0,
        },
    );
    let text = "あいうえお";
    let text = build_input_text(text, 0, 3, CategoryType::KANJI);

    let nodes = plugin
        .provide_oov(&text, 0, false)
        .expect("Failed to generate oovs");
    assert_eq!(1, nodes.len());
    let wi = nodes[0].word_info.as_ref().unwrap();
    assert_eq!("あいう", wi.surface);
    assert_eq!(9, wi.head_word_length);
    assert_eq!(1, wi.pos_id);

    let nodes = plugin
        .provide_oov(&text, 0, true)
        .expect("Failed to generate oovs");
    assert!(nodes.is_empty());
}

#[test]
fn provide_oov110() {
    let mut plugin = build_plugin();
    plugin.categories.insert(
        CategoryType::KANJI,
        CategoryInfo {
            category_type: CategoryType::KANJI,
            is_invoke: true,
            is_group: true,
            length: 0,
        },
    );
    let text = "あいうえお";
    let text = build_input_text(text, 0, 3, CategoryType::KANJI);

    let nodes = plugin
        .provide_oov(&text, 0, false)
        .expect("Failed to generate oovs");
    assert_eq!(1, nodes.len());
    let wi = nodes[0].word_info.as_ref().unwrap();
    assert_eq!("あいう", wi.surface);
    assert_eq!(9, wi.head_word_length);
    assert_eq!(1, wi.pos_id);

    let nodes = plugin
        .provide_oov(&text, 0, true)
        .expect("Failed to generate oovs");
    assert_eq!(1, nodes.len());
}

#[test]
fn provide_oov002() {
    let mut plugin = build_plugin();
    plugin.categories.insert(
        CategoryType::KANJI,
        CategoryInfo {
            category_type: CategoryType::KANJI,
            is_invoke: false,
            is_group: false,
            length: 2,
        },
    );
    let text = "あいうえお";
    let text = build_input_text(text, 0, 3, CategoryType::KANJI);

    let nodes = plugin
        .provide_oov(&text, 0, false)
        .expect("Failed to generate oovs");
    assert_eq!(2, nodes.len());

    let wi = nodes[0].word_info.as_ref().unwrap();
    assert_eq!("あ", wi.surface);
    assert_eq!(3, wi.head_word_length);
    assert_eq!(1, wi.pos_id);

    let wi = nodes[1].word_info.as_ref().unwrap();
    assert_eq!("あい", wi.surface);
    assert_eq!(6, wi.head_word_length);
    assert_eq!(1, wi.pos_id);

    let nodes = plugin
        .provide_oov(&text, 0, true)
        .expect("Failed to generate oovs");
    assert!(nodes.is_empty());
}

#[test]
fn provide_oov012() {
    let mut plugin = build_plugin();
    plugin.categories.insert(
        CategoryType::KANJI,
        CategoryInfo {
            category_type: CategoryType::KANJI,
            is_invoke: false,
            is_group: true,
            length: 2,
        },
    );
    let text = "あいうえお";
    let text = build_input_text(text, 0, 3, CategoryType::KANJI);

    let nodes = plugin
        .provide_oov(&text, 0, false)
        .expect("Failed to generate oovs");
    assert_eq!(3, nodes.len());

    let wi = nodes[0].word_info.as_ref().unwrap();
    assert_eq!("あいう", wi.surface);
    assert_eq!(9, wi.head_word_length);
    assert_eq!(1, wi.pos_id);

    let wi = nodes[1].word_info.as_ref().unwrap();
    assert_eq!("あ", wi.surface);
    assert_eq!(3, wi.head_word_length);
    assert_eq!(1, wi.pos_id);

    let wi = nodes[2].word_info.as_ref().unwrap();
    assert_eq!("あい", wi.surface);
    assert_eq!(6, wi.head_word_length);
    assert_eq!(1, wi.pos_id);

    let nodes = plugin
        .provide_oov(&text, 0, true)
        .expect("Failed to generate oovs");
    assert!(nodes.is_empty());
}

#[test]
fn provide_oov112() {
    let mut plugin = build_plugin();
    plugin.categories.insert(
        CategoryType::KANJI,
        CategoryInfo {
            category_type: CategoryType::KANJI,
            is_invoke: true,
            is_group: true,
            length: 2,
        },
    );
    let text = "あいうえお";
    let text = build_input_text(text, 0, 3, CategoryType::KANJI);

    let nodes = plugin
        .provide_oov(&text, 0, false)
        .expect("Failed to generate oovs");
    assert_eq!(3, nodes.len());

    let wi = nodes[0].word_info.as_ref().unwrap();
    assert_eq!("あいう", wi.surface);
    assert_eq!(9, wi.head_word_length);
    assert_eq!(1, wi.pos_id);

    let wi = nodes[1].word_info.as_ref().unwrap();
    assert_eq!("あ", wi.surface);
    assert_eq!(3, wi.head_word_length);
    assert_eq!(1, wi.pos_id);

    let wi = nodes[2].word_info.as_ref().unwrap();
    assert_eq!("あい", wi.surface);
    assert_eq!(6, wi.head_word_length);
    assert_eq!(1, wi.pos_id);

    let nodes = plugin
        .provide_oov(&text, 0, true)
        .expect("Failed to generate oovs");
    assert_eq!(3, nodes.len());
}

#[test]
fn provide_oov006() {
    let mut plugin = build_plugin();
    plugin.categories.insert(
        CategoryType::KANJI,
        CategoryInfo {
            category_type: CategoryType::KANJI,
            is_invoke: false,
            is_group: false,
            length: 6,
        },
    );
    let text = "あいうえお";
    let text = build_input_text(text, 0, 3, CategoryType::KANJI);

    let nodes = plugin
        .provide_oov(&text, 0, false)
        .expect("Failed to generate oovs");
    assert_eq!(3, nodes.len());

    let wi = nodes[0].word_info.as_ref().unwrap();
    assert_eq!("あ", wi.surface);
    assert_eq!(3, wi.head_word_length);
    assert_eq!(1, wi.pos_id);

    let wi = nodes[1].word_info.as_ref().unwrap();
    assert_eq!("あい", wi.surface);
    assert_eq!(6, wi.head_word_length);
    assert_eq!(1, wi.pos_id);

    let wi = nodes[2].word_info.as_ref().unwrap();
    assert_eq!("あいう", wi.surface);
    assert_eq!(9, wi.head_word_length);
    assert_eq!(1, wi.pos_id);

    let nodes = plugin
        .provide_oov(&text, 0, true)
        .expect("Failed to generate oovs");
    assert!(nodes.is_empty());
}

#[test]
fn provide_oov_multi_oov() {
    let mut plugin = build_plugin();
    plugin.categories.insert(
        CategoryType::KANJINUMERIC,
        CategoryInfo {
            category_type: CategoryType::KANJINUMERIC,
            is_invoke: false,
            is_group: true,
            length: 0,
        },
    );
    let text = "あいうえお";
    let text = build_input_text(text, 0, 3, CategoryType::KANJINUMERIC);

    let nodes = plugin
        .provide_oov(&text, 0, false)
        .expect("Failed to generate oovs");
    assert_eq!(2, nodes.len());

    let wi = nodes[0].word_info.as_ref().unwrap();
    assert_eq!("あいう", wi.surface);
    assert_eq!(9, wi.head_word_length);
    assert_eq!(1, wi.pos_id);

    let wi = nodes[1].word_info.as_ref().unwrap();
    assert_eq!("あいう", wi.surface);
    assert_eq!(9, wi.head_word_length);
    assert_eq!(2, wi.pos_id);
}

#[test]
fn provide_oov_without_cinfo() {
    let plugin = build_plugin();
    let text = "あいうえお";
    let text = build_input_text(text, 0, 3, CategoryType::KANJI);

    let nodes = plugin
        .provide_oov(&text, 0, false)
        .expect("Failed to generate oovs");
    assert!(nodes.is_empty());
}

#[test]
fn provide_oov_without_oov_list() {
    let mut plugin = build_plugin();
    plugin.categories.insert(
        CategoryType::HIRAGANA,
        CategoryInfo {
            category_type: CategoryType::HIRAGANA,
            is_invoke: false,
            is_group: true,
            length: 0,
        },
    );
    let text = "あいうえお";
    let text = build_input_text(text, 0, 3, CategoryType::HIRAGANA);

    let nodes = plugin
        .provide_oov(&text, 0, false)
        .expect("Failed to generate oovs");
    assert!(nodes.is_empty());
}

#[test]
fn read_character_property() {
    let mut file = tempfile().expect("Failed to get temporary file");
    writeln!(file, "#\n").unwrap();
    writeln!(file, "DEFAULT 0 1 2").unwrap();
    writeln!(file, "ALPHA 1 0 0").unwrap();
    writeln!(file, "0x0000...0x0002 ALPHA").unwrap();
    file.flush().expect("Failed to flush");
    file.seek(SeekFrom::Start(0)).expect("Failed to seek");

    let categories = MeCabOovPlugin::read_character_property(BufReader::new(file))
        .expect("Failed to read tmp file");
    assert!(!categories.get(&CategoryType::DEFAULT).unwrap().is_invoke);
    assert!(categories.get(&CategoryType::DEFAULT).unwrap().is_group);
    assert_eq!(2, categories.get(&CategoryType::DEFAULT).unwrap().length);
}

#[test]
fn read_character_property_with_too_few_columns() {
    let data = "DEFAULT 0 1";
    let result = MeCabOovPlugin::read_character_property(data.as_bytes());
    assert_matches!(
        result,
        Err(SudachiError::InvalidCharacterCategory(
            CharacterCategoryError::InvalidFormat(0)
        ))
    );
}

#[test]
fn read_character_property_with_undefined_type() {
    let data = "FOO 0 1 2";
    let result = MeCabOovPlugin::read_character_property(data.as_bytes());
    assert_matches!(result,
        Err(SudachiError::InvalidCharacterCategory(
            CharacterCategoryError::InvalidCategoryType(0, s))) if s == "FOO");
}

#[test]
fn read_character_property_duplicate_definitions() {
    let data = "#
    DEFAULT 0 1 2
    DEFAULT 1 1 2";
    let result = MeCabOovPlugin::read_character_property(data.as_bytes());
    assert_matches!(result,
        Err(SudachiError::InvalidCharacterCategory(
            CharacterCategoryError::MultipleTypeDefinition(2, s))) if s == "DEFAULT");
}

#[test]
fn read_oov() {
    let mut file = tempfile().expect("Failed to get temporary file");
    writeln!(file, "DEFAULT,1,2,3,補助記号,一般,*,*,*,*").unwrap();
    writeln!(file, "DEFAULT,3,4,5,補助記号,一般,*,*,*,*").unwrap();
    file.flush().expect("Failed to flush");
    file.seek(SeekFrom::Start(0)).expect("Failed to seek");

    let bytes = build_mock_bytes();
    let grammar = build_mock_grammar(&bytes);
    let mut categories = HashMap::new();
    categories.insert(
        CategoryType::DEFAULT,
        CategoryInfo {
            category_type: CategoryType::DEFAULT,
            is_invoke: false,
            is_group: false,
            length: 0,
        },
    );
    let oov_list = MeCabOovPlugin::read_oov(BufReader::new(file), &categories, &grammar)
        .expect("Failed to read tmp file");

    assert_eq!(1, oov_list.len());
    assert_eq!(2, oov_list.get(&CategoryType::DEFAULT).unwrap().len());
    assert_eq!(1, oov_list.get(&CategoryType::DEFAULT).unwrap()[0].left_id);
    assert_eq!(2, oov_list.get(&CategoryType::DEFAULT).unwrap()[0].right_id);
    assert_eq!(3, oov_list.get(&CategoryType::DEFAULT).unwrap()[0].cost);
    assert_eq!(0, oov_list.get(&CategoryType::DEFAULT).unwrap()[0].pos_id);
}

#[test]
fn read_oov_with_too_few_columns() {
    let bytes = build_mock_bytes();
    let grammar = build_mock_grammar(&bytes);
    let mut categories = HashMap::new();
    categories.insert(
        CategoryType::DEFAULT,
        CategoryInfo {
            category_type: CategoryType::DEFAULT,
            is_invoke: false,
            is_group: false,
            length: 0,
        },
    );

    let data = "DEFAULT,1,2,3,補助記号,一般,*,*,*";
    let result = MeCabOovPlugin::read_oov(data.as_bytes(), &categories, &grammar);
    assert_matches!(result, Err(SudachiError::InvalidDataFormat(0, s)) if s == data);
}

#[test]
fn read_oov_with_undefined_type() {
    let bytes = build_mock_bytes();
    let grammar = build_mock_grammar(&bytes);
    let mut categories = HashMap::new();
    categories.insert(
        CategoryType::DEFAULT,
        CategoryInfo {
            category_type: CategoryType::DEFAULT,
            is_invoke: false,
            is_group: false,
            length: 0,
        },
    );

    let data = "FOO,1,2,3,補助記号,一般,*,*,*,*";
    let result = MeCabOovPlugin::read_oov(data.as_bytes(), &categories, &grammar);
    assert_matches!(result, Err(SudachiError::InvalidCharacterCategoryType(s)) if s == "FOO")
}

#[test]
fn read_oov_with_category_not_in_character_property() {
    let bytes = build_mock_bytes();
    let grammar = build_mock_grammar(&bytes);
    let mut categories = HashMap::new();
    categories.insert(
        CategoryType::DEFAULT,
        CategoryInfo {
            category_type: CategoryType::DEFAULT,
            is_invoke: false,
            is_group: false,
            length: 0,
        },
    );

    let data = "ALPHA,1,2,3,補助記号,一般,*,*,*,*";
    let result = MeCabOovPlugin::read_oov(data.as_bytes(), &categories, &grammar);
    assert_matches!(result, Err(SudachiError::InvalidDataFormat(0, s)) if s.contains("ALPHA"))
}

fn build_plugin() -> MeCabOovPlugin {
    let mut plugin = MeCabOovPlugin::default();
    let oov1 = OOV {
        right_id: -1,
        left_id: -1,
        cost: -1,
        pos_id: 1,
    };
    let oov2 = OOV {
        right_id: -1,
        left_id: -1,
        cost: -1,
        pos_id: 2,
    };
    plugin
        .oov_list
        .insert(CategoryType::KANJI, vec![oov1.clone()]);
    plugin
        .oov_list
        .insert(CategoryType::KANJINUMERIC, vec![oov1, oov2]);
    plugin
}

fn build_input_text<'a>(
    text: &'a str,
    begin: usize,
    end: usize,
    ctype: CategoryType,
) -> Utf8InputText<'a> {
    // assume text = "あいうえお"
    let offsets = vec![0, 0, 0, 3, 3, 3, 6, 6, 6, 9, 9, 9, 12, 12, 12, 15];
    let byte_indexes = vec![0, 0, 0, 1, 1, 1, 2, 2, 2, 3, 3, 3, 4, 4, 4, 5];
    let mut char_category_types = vec![CategoryTypes::default(); 5];
    for i in begin..end {
        char_category_types[i].insert(ctype);
    }
    let can_bow_list = vec![true; 5];
    let mut char_category_continuities = vec![0; 15];
    for i in begin * 3..end * 3 {
        char_category_continuities[i] = (end - begin) * 3 - i;
    }

    let text = Utf8InputText::new(
        text,
        String::from(text),
        offsets,
        byte_indexes,
        char_category_types,
        can_bow_list,
        char_category_continuities,
    );
    text
}

fn build_mock_bytes() -> Vec<u8> {
    let mut buf = Vec::new();
    // encode pos for oov
    buf.extend(&(1 as i16).to_le_bytes());
    let pos = vec!["補助記号", "一般", "*", "*", "*", "*"];
    for s in pos {
        let utf16: Vec<_> = s.encode_utf16().collect();
        buf.extend(&(utf16.len() as u8).to_le_bytes());
        for c in utf16 {
            buf.extend(&(c).to_le_bytes());
        }
    }
    // set 0 for left and right id size
    buf.extend(&(0 as i16).to_le_bytes());
    buf.extend(&(0 as i16).to_le_bytes());
    buf
}

fn build_mock_grammar(bytes: &[u8]) -> Grammar {
    Grammar::new(bytes, 0).expect("Failed to create grammar")
}
