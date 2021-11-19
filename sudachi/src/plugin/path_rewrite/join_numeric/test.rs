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

use crate::analysis::Node;
use crate::dic::character_category::CharacterCategory;
use crate::dic::grammar::Grammar;
use crate::dic::lexicon::word_infos::WordInfoData;
use crate::dic::word_id::WordId;
use crate::test::zero_grammar;
use lazy_static::lazy_static;

fn build_text(data: &str) -> InputBuffer {
    let mut buf = InputBuffer::from(data);
    buf.build(&GRAMMAR).expect("should not fail");
    buf
}

#[test]
fn digit_1() {
    let plugin = build_plugin();
    let text = build_text("123円20銭");
    let path = vec![
        build_node_num("1", "1", 0, 0),
        build_node_num("2", "2", 1, 1),
        build_node_num("3", "3", 2, 2),
        build_node_oov("円", "円", 3, 3),
        build_node_num("2", "2", 4, 6),
        build_node_num("0", "0", 5, 7),
        build_node_oov("銭", "銭", 6, 8),
    ];
    let path = plugin
        .rewrite(&text, path, &Lattice::default())
        .expect("Failed to rewrite path");
    assert_eq!(4, path.len());
    assert_eq!("123", path[0].word_info().surface());
    assert_eq!("20", path[2].word_info().surface());
}

#[test]
fn digit_2() {
    let plugin = build_plugin();
    let text = build_text("080-121");
    let path = vec![
        build_node_num("0", "0", 0, 0),
        build_node_num("8", "8", 1, 1),
        build_node_num("0", "0", 2, 2),
        build_node_oov("-", "-", 3, 3),
        build_node_num("1", "1", 4, 4),
        build_node_num("2", "2", 5, 5),
        build_node_num("1", "1", 6, 6),
    ];
    let path = plugin
        .rewrite(&text, path, &Lattice::default())
        .expect("Failed to rewrite path");
    assert_eq!(3, path.len());
    assert_eq!("080", path[0].word_info().surface());
    assert_eq!("121", path[2].word_info().surface());
}

#[test]
fn kanji_numeric() {
    let plugin = build_plugin();
    let text = build_text("一二三万二千円");
    let path = vec![
        build_node_num("一", "一", 0, 0),
        build_node_num("二", "二", 1, 3),
        build_node_num("三", "三", 2, 6),
        build_node_oov("万", "万", 3, 9),
        build_node_num("二", "二", 4, 12),
        build_node_oov("千", "千", 5, 15),
        build_node_oov("円", "円", 6, 18),
    ];
    let path = plugin
        .rewrite(&text, path, &Lattice::default())
        .expect("Failed to rewrite path");
    assert_eq!(2, path.len());
    assert_eq!("一二三万二千", path[0].word_info().surface());

    let text = build_text("二百百");
    let path = vec![
        build_node_num("二", "二", 0, 0),
        build_node_oov("百", "百", 1, 3),
        build_node_oov("百", "百", 2, 6),
    ];
    let path = plugin
        .rewrite(&text, path, &Lattice::default())
        .expect("Failed to rewrite path");
    assert_eq!(3, path.len());
}

#[test]
fn normalize() {
    let plugin = build_plugin();
    let text = build_text("一二三万二千円");
    let path = vec![
        build_node_num("一", "一", 0, 0),
        build_node_num("二", "二", 1, 3),
        build_node_num("三", "三", 2, 6),
        build_node_oov("万", "万", 3, 9),
        build_node_num("二", "二", 4, 12),
        build_node_oov("千", "千", 5, 15),
        build_node_oov("円", "円", 6, 18),
    ];
    let path = plugin
        .rewrite(&text, path, &Lattice::default())
        .expect("Failed to rewrite path");
    assert_eq!(2, path.len());
    assert_eq!("1232000", path[0].word_info().normalized_form());
}

#[test]
fn normalized_with_not_numeric() {
    let plugin = build_plugin();
    let text = build_text("六三四");
    // 六三四 is in the dictionary
    let path = vec![build_node_oov("六三四", "六三四", 0, 0)];
    let path = plugin
        .rewrite(&text, path, &Lattice::default())
        .expect("Failed to rewrite path");
    assert_eq!(1, path.len());
    assert_eq!("六三四", path[0].word_info().normalized_form());
}

#[test]
fn point() {
    let plugin = build_plugin();
    let text = build_text("1.002");
    let path = vec![
        build_node_num("1", "1", 0, 0),
        build_node_oov(".", ".", 1, 1),
        build_node_num("0", "0", 2, 2),
        build_node_num("0", "0", 3, 3),
        build_node_num("2", "2", 4, 4),
    ];
    let path = plugin
        .rewrite(&text, path, &Lattice::default())
        .expect("Failed to rewrite path");
    assert_eq!(1, path.len());
    assert_eq!("1.002", path[0].word_info().normalized_form());

    let text = build_text(".002");
    let path = vec![
        build_node_oov(".", ".", 0, 0),
        build_node_num("0", "0", 1, 1),
        build_node_num("0", "0", 2, 2),
        build_node_num("2", "2", 3, 3),
    ];
    let path = plugin
        .rewrite(&text, path, &Lattice::default())
        .expect("Failed to rewrite path");
    assert_eq!(2, path.len());
    assert_eq!(".", path[0].word_info().normalized_form());
    assert_eq!("002", path[1].word_info().normalized_form());

    let text = build_text("22.");
    let path = vec![
        build_node_num("2", "2", 0, 0),
        build_node_num("2", "2", 1, 1),
        build_node_oov(".", ".", 2, 2),
    ];
    let path = plugin
        .rewrite(&text, path, &Lattice::default())
        .expect("Failed to rewrite path");
    assert_eq!(2, path.len());
    assert_eq!("22", path[0].word_info().normalized_form());
    assert_eq!(".", path[1].word_info().normalized_form());

    let text = build_text("22.節");
    let path = vec![
        build_node_num("2", "2", 0, 0),
        build_node_num("2", "2", 1, 1),
        build_node_oov(".", ".", 2, 2),
        build_node_oov("節", "節", 3, 3),
    ];
    let path = plugin
        .rewrite(&text, path, &Lattice::default())
        .expect("Failed to rewrite path");
    assert_eq!(3, path.len());
    assert_eq!("22", path[0].word_info().normalized_form());
    assert_eq!(".", path[1].word_info().normalized_form());

    let text = build_text(".c");
    let path = vec![
        build_node_oov(".", ".", 0, 0),
        build_node_oov("c", "c", 1, 1),
    ];
    let path = plugin
        .rewrite(&text, path, &Lattice::default())
        .expect("Failed to rewrite path");
    assert_eq!(2, path.len());
    assert_eq!(".", path[0].word_info().normalized_form());

    let text = build_text("1.20.3");
    let path = vec![
        build_node_num("1", "1", 0, 0),
        build_node_oov(".", ".", 1, 1),
        build_node_num("2", "2", 2, 2),
        build_node_num("0", "0", 3, 3),
        build_node_oov(".", ".", 4, 4),
        build_node_num("3", "3", 5, 5),
    ];
    let path = plugin
        .rewrite(&text, path, &Lattice::default())
        .expect("Failed to rewrite path");
    assert_eq!(5, path.len());
    assert_eq!("20", path[2].word_info().normalized_form());

    let text = build_text("652...");
    let path = vec![
        build_node_num("6", "6", 0, 0),
        build_node_num("5", "5", 1, 1),
        build_node_num("2", "2", 2, 2),
        build_node_oov(".", ".", 3, 3),
        build_node_oov(".", ".", 4, 4),
        build_node_oov(".", ".", 5, 5),
    ];
    let path = plugin
        .rewrite(&text, path, &Lattice::default())
        .expect("Failed to rewrite path");
    assert_eq!(4, path.len());
    assert_eq!("652", path[0].word_info().normalized_form());
}

#[test]
fn comma() {
    let plugin = build_plugin();
    let text = build_text("2,00,000,000円");
    let path = vec![
        build_node_num("2", "2", 0, 0),
        build_node_oov(",", ",", 1, 1),
        build_node_num("0", "0", 2, 2),
        build_node_num("0", "0", 3, 3),
        build_node_oov(",", ",", 4, 4),
        build_node_num("0", "0", 5, 5),
        build_node_num("0", "0", 6, 6),
        build_node_num("0", "0", 7, 7),
        build_node_oov(",", ",", 8, 8),
        build_node_num("0", "0", 9, 9),
        build_node_num("0", "0", 10, 10),
        build_node_num("0", "0", 11, 11),
        build_node_oov("円", "円", 12, 12),
    ];
    let path = plugin
        .rewrite(&text, path, &Lattice::default())
        .expect("Failed to rewrite path");
    assert_eq!(8, path.len());
    assert_eq!("2", path[0].word_info().normalized_form());
    assert_eq!(",", path[1].word_info().normalized_form());
    assert_eq!("00", path[2].word_info().normalized_form());
    assert_eq!(",", path[3].word_info().normalized_form());
    assert_eq!("000", path[4].word_info().normalized_form());
    assert_eq!(",", path[5].word_info().normalized_form());
    assert_eq!("000", path[6].word_info().normalized_form());

    let text = build_text(",");
    let path = vec![build_node_oov(",", ",", 0, 0)];
    let path = plugin
        .rewrite(&text, path, &Lattice::default())
        .expect("Failed to rewrite path");
    assert_eq!(1, path.len());
    assert_eq!(",", path[0].word_info().normalized_form());

    let text = build_text("652,,,");
    let path = vec![
        build_node_num("6", "6", 0, 0),
        build_node_num("5", "5", 1, 1),
        build_node_num("2", "2", 2, 2),
        build_node_oov(",", ",", 3, 3),
        build_node_oov(",", ",", 4, 4),
        build_node_oov(",", ",", 5, 5),
    ];
    let path = plugin
        .rewrite(&text, path, &Lattice::default())
        .expect("Failed to rewrite path");
    assert_eq!(4, path.len());
    assert_eq!("652", path[0].word_info().normalized_form());

    let text = build_text("256,5.50389");
    let path = vec![
        build_node_num("2", "2", 0, 0),
        build_node_num("5", "5", 1, 1),
        build_node_num("6", "6", 2, 2),
        build_node_oov(",", ",", 3, 3),
        build_node_num("5", "5", 4, 4),
        build_node_oov(".", ".", 5, 5),
        build_node_num("5", "5", 6, 6),
        build_node_num("0", "0", 7, 7),
        build_node_num("3", "3", 8, 8),
        build_node_num("8", "8", 9, 9),
        build_node_num("9", "9", 10, 10),
    ];
    let path = plugin
        .rewrite(&text, path, &Lattice::default())
        .expect("Failed to rewrite path");
    assert_eq!(3, path.len());
    assert_eq!("256", path[0].word_info().normalized_form());
    assert_eq!("5.50389", path[2].word_info().normalized_form());

    let text = build_text("256,550.389");
    let path = vec![
        build_node_num("2", "2", 0, 0),
        build_node_num("5", "5", 1, 1),
        build_node_num("6", "6", 2, 2),
        build_node_oov(",", ",", 3, 3),
        build_node_num("5", "5", 4, 4),
        build_node_num("5", "5", 5, 5),
        build_node_num("0", "0", 6, 6),
        build_node_oov(".", ".", 7, 7),
        build_node_num("3", "3", 8, 8),
        build_node_num("8", "8", 9, 9),
        build_node_num("9", "9", 10, 10),
    ];
    let path = plugin
        .rewrite(&text, path, &Lattice::default())
        .expect("Failed to rewrite path");
    assert_eq!(1, path.len());
    assert_eq!("256550.389", path[0].word_info().normalized_form());
}

#[test]
fn single_node() {
    let mut plugin = build_plugin();
    let text = build_text("猫三匹");
    let _path = vec![
        build_node_oov("猫", "猫", 0, 0),
        build_node_num("三", "三", 1, 3),
        build_node_oov("匹", "匹", 2, 6),
    ];

    let path = plugin
        .rewrite(&text, _path.clone(), &Lattice::default())
        .expect("Failed to rewrite path");
    assert_eq!(3, path.len());
    assert_eq!("3", path[1].word_info().normalized_form());

    plugin.enable_normalize = false;
    let path = plugin
        .rewrite(&text, _path.clone(), &Lattice::default())
        .expect("Failed to rewrite path");
    assert_eq!(3, path.len());
    assert_eq!("三", path[1].word_info().normalized_form());
}

fn build_node_num(surface: &str, normalized: &str, start_cp: usize, start_b: usize) -> ResultNode {
    ResultNode::new(
        Node::new(
            start_cp as u16,
            (start_cp + surface.chars().count()) as u16,
            9,
            9,
            2478,
            WordId::new(0, 1),
        ),
        9,
        start_b as u16,
        (start_b + surface.len()) as u16,
        WordInfoData {
            surface: surface.to_string(),
            head_word_length: surface.len() as u16,
            pos_id: 7,
            normalized_form: normalized.to_string(),
            dictionary_form_word_id: -1,
            dictionary_form: surface.to_string(),
            ..Default::default()
        }
        .into(),
    )
}
fn build_node_oov(surface: &str, normalized: &str, start_cp: usize, start_b: usize) -> ResultNode {
    ResultNode::new(
        Node::new(
            start_cp as u16,
            (start_cp + surface.chars().count()) as u16,
            8,
            8,
            6000,
            WordId::oov(4),
        ),
        9,
        start_b as u16,
        (start_b + surface.len()) as u16,
        WordInfoData {
            surface: surface.to_string(),
            head_word_length: surface.len() as u16,
            pos_id: 4,
            normalized_form: normalized.to_string(),
            dictionary_form_word_id: -1,
            dictionary_form: surface.to_string(),
            ..Default::default()
        }
        .into(),
    )
}

fn build_plugin() -> JoinNumericPlugin {
    JoinNumericPlugin {
        numeric_pos_id: 7,
        enable_normalize: true,
    }
}

const CHAR_DEF: &[u8] = include_bytes!("test_char.def");

fn build_character_category() -> CharacterCategory {
    CharacterCategory::from_reader(CHAR_DEF).expect("Failed to load character category")
}

fn build_mock_grammar() -> Grammar<'static> {
    let mut grammar = zero_grammar();
    let char_cat = build_character_category();
    grammar.set_character_category(char_cat);
    grammar
}

lazy_static! {
    static ref GRAMMAR: Grammar<'static> = build_mock_grammar();
}
