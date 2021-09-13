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

use super::*;

use std::path::PathBuf;

use sudachi::dic::character_category::CharacterCategory;
use sudachi::dic::grammar::Grammar;
use sudachi::dic::lexicon::word_infos::WordInfo;
use sudachi::input_text::utf8_input_text_builder::Utf8InputTextBuilder;

const TEST_RESOURCE_DIR_PATH: &str = "tests/resources/";

#[test]
fn katakana_length() {
    let mut plugin = JoinKarakanaOovPlugin::default();
    let bytes = build_mock_bytes();
    let grammar = build_mock_grammar(&bytes);
    let builder = Utf8InputTextBuilder::new("アイアイウ", &grammar);
    let text = builder.build();
    let _path = vec![build_node_ai(0, 6, 5562), build_node_aiu(6, 15, 12578)];

    plugin.min_length = 0;
    let path = plugin
        .rewrite(&text, _path.clone(), &Lattice::new(&grammar, 0))
        .expect("Failed to rewrite path");
    assert_eq!(2, path.len());

    plugin.min_length = 1;
    let path = plugin
        .rewrite(&text, _path.clone(), &Lattice::new(&grammar, 0))
        .expect("Failed to rewrite path");
    assert_eq!(2, path.len());

    plugin.min_length = 2;
    let path = plugin
        .rewrite(&text, _path.clone(), &Lattice::new(&grammar, 0))
        .expect("Failed to rewrite path");
    assert_eq!(2, path.len());

    plugin.min_length = 3;
    let path = plugin
        .rewrite(&text, _path.clone(), &Lattice::new(&grammar, 0))
        .expect("Failed to rewrite path");
    assert_eq!(1, path.len());
}

#[test]
fn part_of_speech() {
    let mut plugin = JoinKarakanaOovPlugin::default();
    let bytes = build_mock_bytes();
    let grammar = build_mock_grammar(&bytes);
    let builder = Utf8InputTextBuilder::new("アイアイウ", &grammar);
    let text = builder.build();
    let path = vec![build_node_ai(0, 6, 5562), build_node_aiu(6, 15, 12578)];

    plugin.min_length = 3;
    let path = plugin
        .rewrite(&text, path, &Lattice::new(&grammar, 0))
        .expect("Failed to rewrite path");
    assert_eq!(1, path.len());
    assert!(!path[0].is_oov);
}

#[test]
fn start_with_middle() {
    let mut plugin = JoinKarakanaOovPlugin::default();
    let bytes = build_mock_bytes();
    let grammar = build_mock_grammar(&bytes);
    let builder = Utf8InputTextBuilder::new("アイウアイアイウ", &grammar);
    let text = builder.build();
    let path = vec![
        build_node_aiu(0, 9, 5562),
        build_node_ai(9, 15, 12578),
        build_node_aiu(15, 24, 19594),
    ];

    plugin.min_length = 3;
    let path = plugin
        .rewrite(&text, path, &Lattice::new(&grammar, 0))
        .expect("Failed to rewrite path");
    assert_eq!(1, path.len());
}

#[test]
fn start_with_tail() {
    let mut plugin = JoinKarakanaOovPlugin::default();
    let bytes = build_mock_bytes();
    let grammar = build_mock_grammar(&bytes);
    let builder = Utf8InputTextBuilder::new("アイウアイウアイ", &grammar);
    let text = builder.build();
    let path = vec![
        build_node_aiu(0, 9, 5562),
        build_node_aiu(9, 18, 12578),
        build_node_ai(18, 24, 19594),
    ];

    plugin.min_length = 3;
    let path = plugin
        .rewrite(&text, path, &Lattice::new(&grammar, 0))
        .expect("Failed to rewrite path");
    assert_eq!(1, path.len());
}

#[test]
fn with_noovbow() {
    let bytes = build_mock_bytes();
    let grammar = build_mock_grammar(&bytes);
    let mut plugin = JoinKarakanaOovPlugin::default();
    plugin.min_length = 3;

    let builder = Utf8InputTextBuilder::new("ァアイアイウ", &grammar);
    let text = builder.build();
    let path = vec![
        build_node_oov(0, 3, 6447, "ァ", 3),
        build_node_aiu(3, 9, 13969),
        build_node_ai(9, 18, 20985),
    ];
    let path = plugin
        .rewrite(&text, path, &Lattice::new(&grammar, 0))
        .expect("Failed to rewrite path");
    assert_eq!(2, path.len());
    assert_eq!("ァ", path[0].word_info.clone().unwrap().surface);

    let builder = Utf8InputTextBuilder::new("アイウァアイウ", &grammar);
    let text = builder.build();
    let path = vec![
        build_node_aiu(0, 9, 5562),
        build_node_oov(9, 12, 13613, "ァ", 3),
        build_node_aiu(12, 21, 21135),
    ];
    let path = plugin
        .rewrite(&text, path, &Lattice::new(&grammar, 0))
        .expect("Failed to rewrite path");
    assert_eq!(1, path.len());
}

fn build_node_ai(start: usize, end: usize, cost: i32) -> Node {
    let mut node_ai = Node::new(7, 7, 4675, 10);
    node_ai.set_word_info(WordInfo {
        surface: "アイ".to_string(),
        head_word_length: 6,
        pos_id: 4,
        normalized_form: "アイ".to_string(),
        dictionary_form_word_id: -1,
        dictionary_form: "アイ".to_string(),
        ..WordInfo::default()
    });
    node_ai.set_range(start, end);
    node_ai.total_cost = cost;
    node_ai
}
fn build_node_aiu(start: usize, end: usize, cost: i32) -> Node {
    let mut node_ai = Node::new(7, 7, 4675, 11);
    node_ai.set_word_info(WordInfo {
        surface: "アイウ".to_string(),
        head_word_length: 9,
        pos_id: 4,
        normalized_form: "アイウ".to_string(),
        dictionary_form_word_id: -1,
        dictionary_form: "アイウ".to_string(),
        ..WordInfo::default()
    });
    node_ai.set_range(start, end);
    node_ai.total_cost = cost;
    node_ai
}
fn build_node_oov(start: usize, end: usize, cost: i32, surface: &str, length: u16) -> Node {
    let mut node_ai = Node::new_oov(
        8,
        8,
        6000,
        WordInfo {
            surface: surface.to_string(),
            head_word_length: length,
            pos_id: 4,
            normalized_form: surface.to_string(),
            dictionary_form_word_id: -1,
            dictionary_form: surface.to_string(),
            ..WordInfo::default()
        },
    );
    node_ai.set_range(start, end);
    node_ai.total_cost = cost;
    node_ai
}

fn build_character_category() -> CharacterCategory {
    let char_cat_file_path = PathBuf::from(TEST_RESOURCE_DIR_PATH.to_string() + "char.def");
    CharacterCategory::from_file(&char_cat_file_path).expect("Failed to load character category")
}
fn build_mock_bytes() -> Vec<u8> {
    let mut buf = Vec::new();
    // set 0 for all of pos size, left and right id size
    buf.extend(&(0 as i16).to_le_bytes());
    buf.extend(&(0 as i16).to_le_bytes());
    buf.extend(&(0 as i16).to_le_bytes());
    buf
}
fn build_mock_grammar(bytes: &[u8]) -> Grammar {
    let mut grammar = Grammar::new(bytes, 0).expect("Failed to create grammar");
    let char_cat = build_character_category();
    grammar.set_character_category(char_cat);
    grammar
}
