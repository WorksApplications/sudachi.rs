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
fn digit() {
    let plugin = build_plugin();
    let bytes = build_mock_bytes();
    let grammar = build_mock_grammar(&bytes);

    let builder = Utf8InputTextBuilder::new("123円20銭", &grammar);
    let text = builder.build();
    let path = vec![
        build_node_num("1", "1", 1, 0),
        build_node_num("2", "2", 1, 1),
        build_node_num("3", "3", 1, 2),
        build_node_oov("円", "円", 3, 3),
        build_node_num("2", "2", 1, 6),
        build_node_num("0", "0", 1, 7),
        build_node_oov("銭", "銭", 3, 8),
    ];
    let path = plugin
        .rewrite(&text, path, &Lattice::new(&grammar, 0))
        .expect("Failed to rewrite path");
    assert_eq!(4, path.len());
    assert_eq!("123", path[0].word_info.as_ref().unwrap().surface);
    assert_eq!("20", path[2].word_info.as_ref().unwrap().surface);

    let builder = Utf8InputTextBuilder::new("080-121", &grammar);
    let text = builder.build();
    let path = vec![
        build_node_num("0", "0", 1, 0),
        build_node_num("8", "8", 1, 1),
        build_node_num("0", "0", 1, 2),
        build_node_oov("-", "-", 1, 3),
        build_node_num("1", "1", 1, 4),
        build_node_num("2", "2", 1, 5),
        build_node_num("1", "1", 1, 6),
    ];
    let path = plugin
        .rewrite(&text, path, &Lattice::new(&grammar, 0))
        .expect("Failed to rewrite path");
    assert_eq!(3, path.len());
    assert_eq!("080", path[0].word_info.as_ref().unwrap().surface);
    assert_eq!("121", path[2].word_info.as_ref().unwrap().surface);
}

#[test]
fn kanji_numeric() {
    let plugin = build_plugin();
    let bytes = build_mock_bytes();
    let grammar = build_mock_grammar(&bytes);

    let builder = Utf8InputTextBuilder::new("一二三万二千円", &grammar);
    let text = builder.build();
    let path = vec![
        build_node_num("一", "一", 3, 0),
        build_node_num("二", "二", 3, 3),
        build_node_num("三", "三", 3, 6),
        build_node_oov("万", "万", 3, 9),
        build_node_num("二", "二", 3, 12),
        build_node_oov("千", "千", 3, 15),
        build_node_oov("円", "円", 3, 18),
    ];
    let path = plugin
        .rewrite(&text, path, &Lattice::new(&grammar, 0))
        .expect("Failed to rewrite path");
    assert_eq!(2, path.len());
    assert_eq!("一二三万二千", path[0].word_info.as_ref().unwrap().surface);

    let builder = Utf8InputTextBuilder::new("二百百", &grammar);
    let text = builder.build();
    let path = vec![
        build_node_num("二", "二", 3, 0),
        build_node_oov("百", "百", 3, 3),
        build_node_oov("百", "百", 3, 6),
    ];
    let path = plugin
        .rewrite(&text, path, &Lattice::new(&grammar, 0))
        .expect("Failed to rewrite path");
    assert_eq!(3, path.len());
}

#[test]
fn normalize() {
    let plugin = build_plugin();
    let bytes = build_mock_bytes();
    let grammar = build_mock_grammar(&bytes);

    let builder = Utf8InputTextBuilder::new("一二三万二千円", &grammar);
    let text = builder.build();
    let path = vec![
        build_node_num("一", "一", 3, 0),
        build_node_num("二", "二", 3, 3),
        build_node_num("三", "三", 3, 6),
        build_node_oov("万", "万", 3, 9),
        build_node_num("二", "二", 3, 12),
        build_node_oov("千", "千", 3, 15),
        build_node_oov("円", "円", 3, 18),
    ];
    let path = plugin
        .rewrite(&text, path, &Lattice::new(&grammar, 0))
        .expect("Failed to rewrite path");
    assert_eq!(2, path.len());
    assert_eq!(
        "1232000",
        path[0].word_info.as_ref().unwrap().normalized_form
    );
}

#[test]
fn normalized_with_not_numeric() {
    let plugin = build_plugin();
    let bytes = build_mock_bytes();
    let grammar = build_mock_grammar(&bytes);

    let builder = Utf8InputTextBuilder::new("六三四", &grammar);
    let text = builder.build();
    // 六三四 is in the dictionary
    let path = vec![build_node_oov("六三四", "六三四", 9, 0)];
    let path = plugin
        .rewrite(&text, path, &Lattice::new(&grammar, 0))
        .expect("Failed to rewrite path");
    assert_eq!(1, path.len());
    assert_eq!(
        "六三四",
        path[0].word_info.as_ref().unwrap().normalized_form
    );
}

#[test]
fn point() {
    let plugin = build_plugin();
    let bytes = build_mock_bytes();
    let grammar = build_mock_grammar(&bytes);

    let builder = Utf8InputTextBuilder::new("1.002", &grammar);
    let text = builder.build();
    let path = vec![
        build_node_num("1", "1", 1, 0),
        build_node_oov(".", ".", 1, 1),
        build_node_num("0", "0", 1, 2),
        build_node_num("0", "0", 1, 3),
        build_node_num("2", "2", 1, 4),
    ];
    let path = plugin
        .rewrite(&text, path, &Lattice::new(&grammar, 0))
        .expect("Failed to rewrite path");
    assert_eq!(1, path.len());
    assert_eq!("1.002", path[0].word_info.as_ref().unwrap().normalized_form);

    let builder = Utf8InputTextBuilder::new(".002", &grammar);
    let text = builder.build();
    let path = vec![
        build_node_oov(".", ".", 1, 0),
        build_node_num("0", "0", 1, 1),
        build_node_num("0", "0", 1, 2),
        build_node_num("2", "2", 1, 3),
    ];
    let path = plugin
        .rewrite(&text, path, &Lattice::new(&grammar, 0))
        .expect("Failed to rewrite path");
    assert_eq!(2, path.len());
    assert_eq!(".", path[0].word_info.as_ref().unwrap().normalized_form);
    assert_eq!("002", path[1].word_info.as_ref().unwrap().normalized_form);

    let builder = Utf8InputTextBuilder::new("22.", &grammar);
    let text = builder.build();
    let path = vec![
        build_node_num("2", "2", 1, 0),
        build_node_num("2", "2", 1, 1),
        build_node_oov(".", ".", 1, 2),
    ];
    let path = plugin
        .rewrite(&text, path, &Lattice::new(&grammar, 0))
        .expect("Failed to rewrite path");
    assert_eq!(2, path.len());
    assert_eq!("22", path[0].word_info.as_ref().unwrap().normalized_form);
    assert_eq!(".", path[1].word_info.as_ref().unwrap().normalized_form);

    let builder = Utf8InputTextBuilder::new("22.節", &grammar);
    let text = builder.build();
    let path = vec![
        build_node_num("2", "2", 1, 0),
        build_node_num("2", "2", 1, 1),
        build_node_oov(".", ".", 1, 2),
        build_node_oov("節", "節", 3, 3),
    ];
    let path = plugin
        .rewrite(&text, path, &Lattice::new(&grammar, 0))
        .expect("Failed to rewrite path");
    assert_eq!(3, path.len());
    assert_eq!("22", path[0].word_info.as_ref().unwrap().normalized_form);
    assert_eq!(".", path[1].word_info.as_ref().unwrap().normalized_form);

    let builder = Utf8InputTextBuilder::new(".c", &grammar);
    let text = builder.build();
    let path = vec![
        build_node_oov(".", ".", 1, 0),
        build_node_oov("c", "c", 1, 1),
    ];
    let path = plugin
        .rewrite(&text, path, &Lattice::new(&grammar, 0))
        .expect("Failed to rewrite path");
    assert_eq!(2, path.len());
    assert_eq!(".", path[0].word_info.as_ref().unwrap().normalized_form);

    let builder = Utf8InputTextBuilder::new("1.20.3", &grammar);
    let text = builder.build();
    let path = vec![
        build_node_num("1", "1", 1, 0),
        build_node_oov(".", ".", 1, 1),
        build_node_num("2", "2", 1, 2),
        build_node_num("0", "0", 1, 3),
        build_node_oov(".", ".", 1, 4),
        build_node_num("3", "3", 1, 5),
    ];
    let path = plugin
        .rewrite(&text, path, &Lattice::new(&grammar, 0))
        .expect("Failed to rewrite path");
    assert_eq!(5, path.len());
    assert_eq!("20", path[2].word_info.as_ref().unwrap().normalized_form);

    let builder = Utf8InputTextBuilder::new("652...", &grammar);
    let text = builder.build();
    let path = vec![
        build_node_num("6", "6", 1, 0),
        build_node_num("5", "5", 1, 1),
        build_node_num("2", "2", 1, 2),
        build_node_oov(".", ".", 1, 3),
        build_node_oov(".", ".", 1, 4),
        build_node_oov(".", ".", 1, 5),
    ];
    let path = plugin
        .rewrite(&text, path, &Lattice::new(&grammar, 0))
        .expect("Failed to rewrite path");
    assert_eq!(4, path.len());
    assert_eq!("652", path[0].word_info.as_ref().unwrap().normalized_form);
}

#[test]
fn comma() {
    let plugin = build_plugin();
    let bytes = build_mock_bytes();
    let grammar = build_mock_grammar(&bytes);

    let builder = Utf8InputTextBuilder::new("2,00,000,000円", &grammar);
    let text = builder.build();
    let path = vec![
        build_node_num("2", "2", 1, 0),
        build_node_oov(",", ",", 1, 1),
        build_node_num("0", "0", 1, 2),
        build_node_num("0", "0", 1, 3),
        build_node_oov(",", ",", 1, 4),
        build_node_num("0", "0", 1, 5),
        build_node_num("0", "0", 1, 6),
        build_node_num("0", "0", 1, 7),
        build_node_oov(",", ",", 1, 8),
        build_node_num("0", "0", 1, 9),
        build_node_num("0", "0", 1, 10),
        build_node_num("0", "0", 1, 11),
        build_node_oov("円", "円", 3, 12),
    ];
    let path = plugin
        .rewrite(&text, path, &Lattice::new(&grammar, 0))
        .expect("Failed to rewrite path");
    assert_eq!(8, path.len());
    assert_eq!("2", path[0].word_info.as_ref().unwrap().normalized_form);
    assert_eq!(",", path[1].word_info.as_ref().unwrap().normalized_form);
    assert_eq!("00", path[2].word_info.as_ref().unwrap().normalized_form);
    assert_eq!(",", path[3].word_info.as_ref().unwrap().normalized_form);
    assert_eq!("000", path[4].word_info.as_ref().unwrap().normalized_form);
    assert_eq!(",", path[5].word_info.as_ref().unwrap().normalized_form);
    assert_eq!("000", path[6].word_info.as_ref().unwrap().normalized_form);

    let builder = Utf8InputTextBuilder::new(",", &grammar);
    let text = builder.build();
    let path = vec![build_node_oov(",", ",", 1, 0)];
    let path = plugin
        .rewrite(&text, path, &Lattice::new(&grammar, 0))
        .expect("Failed to rewrite path");
    assert_eq!(1, path.len());
    assert_eq!(",", path[0].word_info.as_ref().unwrap().normalized_form);

    let builder = Utf8InputTextBuilder::new("652,,,", &grammar);
    let text = builder.build();
    let path = vec![
        build_node_num("6", "6", 1, 0),
        build_node_num("5", "5", 1, 1),
        build_node_num("2", "2", 1, 2),
        build_node_oov(",", ",", 1, 3),
        build_node_oov(",", ",", 1, 4),
        build_node_oov(",", ",", 1, 5),
    ];
    let path = plugin
        .rewrite(&text, path, &Lattice::new(&grammar, 0))
        .expect("Failed to rewrite path");
    assert_eq!(4, path.len());
    assert_eq!("652", path[0].word_info.as_ref().unwrap().normalized_form);

    let builder = Utf8InputTextBuilder::new("256,5.50389", &grammar);
    let text = builder.build();
    let path = vec![
        build_node_num("2", "2", 1, 0),
        build_node_num("5", "5", 1, 1),
        build_node_num("6", "6", 1, 2),
        build_node_oov(",", ",", 1, 3),
        build_node_num("5", "5", 1, 4),
        build_node_oov(".", ".", 1, 5),
        build_node_num("5", "5", 1, 6),
        build_node_num("0", "0", 1, 7),
        build_node_num("3", "3", 1, 8),
        build_node_num("8", "8", 1, 9),
        build_node_num("9", "9", 1, 10),
    ];
    let path = plugin
        .rewrite(&text, path, &Lattice::new(&grammar, 0))
        .expect("Failed to rewrite path");
    assert_eq!(3, path.len());
    assert_eq!("256", path[0].word_info.as_ref().unwrap().normalized_form);
    assert_eq!(
        "5.50389",
        path[2].word_info.as_ref().unwrap().normalized_form
    );

    let builder = Utf8InputTextBuilder::new("256,550.389", &grammar);
    let text = builder.build();
    let path = vec![
        build_node_num("2", "2", 1, 0),
        build_node_num("5", "5", 1, 1),
        build_node_num("6", "6", 1, 2),
        build_node_oov(",", ",", 1, 3),
        build_node_num("5", "5", 1, 4),
        build_node_num("5", "5", 1, 5),
        build_node_num("0", "0", 1, 6),
        build_node_oov(".", ".", 1, 7),
        build_node_num("3", "3", 1, 8),
        build_node_num("8", "8", 1, 9),
        build_node_num("9", "9", 1, 10),
    ];
    let path = plugin
        .rewrite(&text, path, &Lattice::new(&grammar, 0))
        .expect("Failed to rewrite path");
    assert_eq!(1, path.len());
    assert_eq!(
        "256550.389",
        path[0].word_info.as_ref().unwrap().normalized_form
    );
}

#[test]
fn single_node() {
    let mut plugin = build_plugin();
    let bytes = build_mock_bytes();
    let grammar = build_mock_grammar(&bytes);

    let builder = Utf8InputTextBuilder::new("猫三匹", &grammar);
    let text = builder.build();
    let _path = vec![
        build_node_oov("猫", "猫", 3, 0),
        build_node_num("三", "三", 3, 3),
        build_node_oov("匹", "匹", 3, 6),
    ];

    let path = plugin
        .rewrite(&text, _path.clone(), &Lattice::new(&grammar, 0))
        .expect("Failed to rewrite path");
    assert_eq!(3, path.len());
    assert_eq!("3", path[1].word_info.as_ref().unwrap().normalized_form);

    plugin.enable_normalize = false;
    let path = plugin
        .rewrite(&text, _path.clone(), &Lattice::new(&grammar, 0))
        .expect("Failed to rewrite path");
    assert_eq!(3, path.len());
    assert_eq!("三", path[1].word_info.as_ref().unwrap().normalized_form);
}

fn build_node_num(surface: &str, normalized: &str, length: u16, start: usize) -> Node {
    let mut node_ai = Node::new_oov(
        9,
        9,
        2478,
        WordInfo {
            surface: surface.to_string(),
            head_word_length: length,
            pos_id: 7,
            normalized_form: normalized.to_string(),
            dictionary_form_word_id: -1,
            dictionary_form: surface.to_string(),
            ..WordInfo::default()
        },
    );
    node_ai.set_range(start, start + length as usize);
    node_ai
}
fn build_node_oov(surface: &str, normalized: &str, length: u16, start: usize) -> Node {
    let mut node_ai = Node::new_oov(
        8,
        8,
        6000,
        WordInfo {
            surface: surface.to_string(),
            head_word_length: length,
            pos_id: 4,
            normalized_form: normalized.to_string(),
            dictionary_form_word_id: -1,
            dictionary_form: surface.to_string(),
            ..WordInfo::default()
        },
    );
    node_ai.set_range(start, start + length as usize);
    node_ai
}

fn build_plugin() -> JoinNumericPlugin {
    JoinNumericPlugin {
        numeric_pos_id: 7,
        enable_normalize: true,
    }
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
