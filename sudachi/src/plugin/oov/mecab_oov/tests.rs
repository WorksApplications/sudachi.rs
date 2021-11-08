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

use crate::analysis::node::LatticeNode;
use claim::assert_matches;
use lazy_static::lazy_static;
use tempfile::tempfile;

use crate::dic::character_category::CharacterCategory;

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

    let text = input_text("あいうcd");
    let mut res: Vec<Node> = vec![];

    plugin
        .provide_oov(&text, 0, false, &mut res)
        .expect("Failed to generate oovs");
    assert!(res.is_empty());

    plugin
        .provide_oov(&text, 0, true, &mut res)
        .expect("Failed to generate oovs");
    assert!(res.is_empty());
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

    let text = input_text("あいうf");
    let mut nodes: Vec<Node> = vec![];

    plugin
        .provide_oov(&text, 0, false, &mut nodes)
        .expect("Failed to generate oovs");
    assert!(nodes.is_empty());

    plugin
        .provide_oov(&text, 0, true, &mut nodes)
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
    let text = input_text("あいうeo");
    let mut nodes: Vec<Node> = vec![];

    plugin
        .provide_oov(&text, 0, false, &mut nodes)
        .expect("Failed to generate oovs");
    assert_eq!(1, nodes.len());
    assert_eq!("あいう", text.curr_slice_c(nodes[0].char_range()));
    assert_eq!(WordId::oov(1), nodes[0].word_id());

    nodes.clear();
    plugin
        .provide_oov(&text, 0, true, &mut nodes)
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
    let text = input_text("あいうeo");
    let mut nodes: Vec<Node> = vec![];

    plugin
        .provide_oov(&text, 0, false, &mut nodes)
        .expect("Failed to generate oovs");
    assert_eq!(1, nodes.len());
    assert_eq!("あいう", text.curr_slice_c(nodes[0].char_range()));
    assert_eq!(WordId::oov(1), nodes[0].word_id());

    nodes.clear();
    plugin
        .provide_oov(&text, 0, true, &mut nodes)
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
    let text = input_text("あいうeo");
    let mut nodes: Vec<Node> = vec![];

    plugin
        .provide_oov(&text, 0, false, &mut nodes)
        .expect("Failed to generate oovs");
    assert_eq!(2, nodes.len());

    assert_eq!("あ", text.curr_slice_c(nodes[0].char_range()));
    assert_eq!(WordId::oov(1), nodes[0].word_id());

    assert_eq!("あい", text.curr_slice_c(nodes[1].char_range()));
    assert_eq!(WordId::oov(1), nodes[1].word_id());

    nodes.clear();
    plugin
        .provide_oov(&text, 0, true, &mut nodes)
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
    let text = input_text("あいうeo");
    let mut nodes: Vec<Node> = vec![];

    plugin
        .provide_oov(&text, 0, false, &mut nodes)
        .expect("Failed to generate oovs");
    assert_eq!(3, nodes.len());

    assert_eq!("あいう", text.curr_slice_c(nodes[0].char_range()));
    assert_eq!(WordId::oov(1), nodes[0].word_id());

    assert_eq!("あ", text.curr_slice_c(nodes[1].char_range()));
    assert_eq!(WordId::oov(1), nodes[1].word_id());

    assert_eq!("あい", text.curr_slice_c(nodes[2].char_range()));
    assert_eq!(WordId::oov(1), nodes[0].word_id());

    nodes.clear();
    plugin
        .provide_oov(&text, 0, true, &mut nodes)
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
    let text = input_text("あいうeo");
    let mut nodes: Vec<Node> = vec![];

    plugin
        .provide_oov(&text, 0, false, &mut nodes)
        .expect("Failed to generate oovs");
    assert_eq!(3, nodes.len());

    assert_eq!("あいう", text.curr_slice_c(nodes[0].char_range()));
    assert_eq!(WordId::oov(1), nodes[0].word_id());

    assert_eq!("あ", text.curr_slice_c(nodes[1].char_range()));
    assert_eq!(WordId::oov(1), nodes[1].word_id());

    assert_eq!("あい", text.curr_slice_c(nodes[2].char_range()));
    assert_eq!(WordId::oov(1), nodes[2].word_id());

    nodes.clear();
    plugin
        .provide_oov(&text, 0, true, &mut nodes)
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
    let text = input_text("あいうeo");
    let mut nodes: Vec<Node> = vec![];

    plugin
        .provide_oov(&text, 0, false, &mut nodes)
        .expect("Failed to generate oovs");
    assert_eq!(3, nodes.len());

    assert_eq!("あ", text.curr_slice_c(nodes[0].char_range()));
    assert_eq!(WordId::oov(1), nodes[0].word_id());

    assert_eq!("あい", text.curr_slice_c(nodes[1].char_range()));
    assert_eq!(WordId::oov(1), nodes[1].word_id());

    assert_eq!("あいう", text.curr_slice_c(nodes[2].char_range()));
    assert_eq!(WordId::oov(1), nodes[2].word_id());

    nodes.clear();
    plugin
        .provide_oov(&text, 0, true, &mut nodes)
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
    let text = input_text("アイウeo");
    let mut nodes: Vec<Node> = vec![];

    plugin
        .provide_oov(&text, 0, false, &mut nodes)
        .expect("Failed to generate oovs");
    assert_eq!(2, nodes.len());

    assert_eq!("アイウ", text.curr_slice_c(nodes[0].char_range()));
    assert_eq!(WordId::oov(1), nodes[0].word_id());

    assert_eq!("アイウ", text.curr_slice_c(nodes[1].char_range()));
    assert_eq!(WordId::oov(2), nodes[1].word_id());
}

#[test]
fn provide_oov_without_cinfo() {
    let plugin = build_plugin();
    let text = input_text("あいうeo");
    let mut nodes: Vec<Node> = vec![];

    plugin
        .provide_oov(&text, 0, false, &mut nodes)
        .expect("Failed to generate oovs");
    assert!(nodes.is_empty());
}

#[test]
fn provide_oov_without_oov_list() {
    let mut plugin = build_plugin();
    plugin.categories.insert(
        CategoryType::ALPHA,
        CategoryInfo {
            category_type: CategoryType::ALPHA,
            is_invoke: false,
            is_group: true,
            length: 0,
        },
    );
    let text = input_text("あいうeo");
    let mut nodes: Vec<Node> = vec![];

    plugin
        .provide_oov(&text, 0, false, &mut nodes)
        .expect("Failed to generate oovs");
    assert!(nodes.is_empty());
}

#[test]
fn read_character_property() {
    let data = "
        DEFAULT 0 1 2
        ALPHA 1 0 0
        0x0000...0x0002 ALPHA
    ";
    let categories =
        MeCabOovPlugin::read_character_property(data.as_bytes()).expect("Failed to read tmp file");
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
    let mut categories = HashMap::with_hasher(RoMu::new());
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
    let mut categories = HashMap::with_hasher(RoMu::new());
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
    let mut categories = HashMap::with_hasher(RoMu::new());
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
    let mut categories = HashMap::with_hasher(RoMu::new());
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

lazy_static! {
    static ref DATA: Vec<u8> = build_mock_bytes();
    static ref GRAMMAR: Grammar<'static> = build_mock_grammar(&DATA);
}

fn input_text(data: &str) -> InputBuffer {
    let mut buf = InputBuffer::from(data);
    buf.build(&GRAMMAR).expect("does not fail");
    buf
}

const ALL_KANJI_CAT: &str = "
0x0061..0x007A ALPHA    #a-z
0x3041..0x309F  KANJI # HIRAGANA
0x30A1..0x30FF  KANJINUMERIC # KATAKANA
";

fn char_cats() -> CharacterCategory {
    CharacterCategory::from_reader(ALL_KANJI_CAT.as_bytes()).unwrap()
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
    // set 10 for left and right id sizes
    buf.extend(&(10 as i16).to_le_bytes());
    buf.extend(&(10 as i16).to_le_bytes());
    for i in 0..10 {
        for j in 0..10 {
            let val = i * 100 + j;
            buf.extend(&(val as i16).to_le_bytes());
        }
    }

    buf
}

fn build_mock_grammar(bytes: &[u8]) -> Grammar {
    let mut grammar = Grammar::parse(bytes, 0).expect("Failed to create grammar");
    grammar.set_character_category(char_cats());
    grammar
}
