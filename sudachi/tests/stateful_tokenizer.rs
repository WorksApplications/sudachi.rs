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

extern crate lazy_static;
extern crate sudachi;

use sudachi::analysis::node::LatticeNode;
use sudachi::prelude::Mode;

mod common;
use crate::common::TestStatefulTokenizer as TestTokenizer;

#[test]
fn empty() {
    let mut tok = TestTokenizer::new_built(Mode::C);
    let ms = tok.tokenize("");
    assert_eq!(0, ms.len());
}

#[test]
fn tokenize_small_katakana_only() {
    let mut tok = TestTokenizer::new_built(Mode::C);
    let ms = tok.tokenize("ァ");
    assert_eq!(1, ms.len());
}

#[test]
fn get_word_id() {
    let mut tok = TestTokenizer::new_built(Mode::C);
    let ms = tok.tokenize("京都");
    assert_eq!(1, ms.len());
    let m0 = ms.get(0);
    let pos = m0.part_of_speech().expect("failed to get pos");
    assert_eq!(&["名詞", "固有名詞", "地名", "一般", "*", "*"], pos);

    // we do not have word_id field in Morpheme and skip testing.
    let ms = tok.tokenize("ぴらる");
    assert_eq!(1, ms.len());
    let m0 = ms.get(0);
    let pos = m0.part_of_speech().expect("failed to get pos");
    assert_eq!(&["名詞", "普通名詞", "一般", "*", "*", "*"], pos);
}

#[test]
fn get_dictionary_id() {
    let mut tok = TestTokenizer::new_built(Mode::C);
    let ms = tok.tokenize("京都");
    let ms: Vec<_> = ms.iter().collect();
    assert_eq!(1, ms.len());
    assert_eq!(0, ms[0].dictionary_id());

    let ms = tok.tokenize("ぴらる");
    let ms: Vec<_> = ms.iter().collect();
    assert_eq!(1, ms.len());
    assert_eq!(1, ms[0].dictionary_id());

    let ms = tok.tokenize("京");
    let ms: Vec<_> = ms.iter().collect();
    assert_eq!(1, ms.len());
    assert!(ms[0].dictionary_id() < 0);
}

#[test]
fn get_synonym_group_id() {
    let mut tok = TestTokenizer::new_built(Mode::C);
    let ms = tok.tokenize("京都");
    assert_eq!(1, ms.len());
    assert_eq!([1, 5], ms.get(0).synonym_group_ids());

    let ms = tok.tokenize("ぴらる");
    assert_eq!(1, ms.len());
    assert!(ms.get(0).synonym_group_ids().is_empty());

    let ms = tok.tokenize("東京府");
    assert_eq!(1, ms.len());
    assert_eq!([1, 3], ms.get(0).synonym_group_ids());
}

#[test]
fn tokenize_kanji_alphabet_word() {
    let mut tok = TestTokenizer::new_built(Mode::C);
    assert_eq!(1, tok.tokenize("特a").len());
    assert_eq!(1, tok.tokenize("ab").len());
    assert_eq!(2, tok.tokenize("特ab").len());
}

#[test]
fn tokenize_with_dots() {
    let mut tok = TestTokenizer::new_built(Mode::C);
    let ms = tok.tokenize("京都…");
    assert_eq!(4, ms.len());
    assert_eq!("…", ms.get(1).surface());
    assert_eq!(".", ms.get(1).normalized_form());
    assert_eq!("", ms.get(2).surface());
    assert_eq!(".", ms.get(2).normalized_form());
    assert_eq!("", ms.get(3).surface());
    assert_eq!(".", ms.get(3).normalized_form());
}

#[test]
fn tokenizer_morpheme_split() {
    let mut tok = TestTokenizer::new_built(Mode::C);
    let ms = tok.tokenize("東京都");
    assert_eq!(1, ms.len());
    assert_eq!("東京都", ms.get(0).surface());

    tok.set_mode(Mode::A);
    let ms = tok.tokenize("東京都");
    assert_eq!(2, ms.len());
    assert_eq!("東京", ms.get(0).surface());
    assert_eq!("都", ms.get(1).surface());
}

#[test]
fn split_middle() {
    let mut tok = TestTokenizer::new_built(Mode::C);
    let ms = tok.tokenize("京都東京都京都");
    assert_eq!(ms.len(), 3);
    let m = ms.get(1);
    assert_eq!(m.surface(), "東京都");

    let ms_a = m.split(Mode::A).expect("works");
    assert_eq!(ms_a.len(), 2);
    assert_eq!(ms_a.get(0).surface(), "東京");
    assert_eq!(ms_a.get_node(0).begin(), 2);
    assert_eq!(ms_a.get_node(0).end(), 4);
    assert_eq!(ms_a.get(0).begin(), 6);
    assert_eq!(ms_a.get(0).end(), 12);
    assert_eq!(ms_a.get(1).surface(), "都");
    assert_eq!(ms_a.get_node(1).begin(), 4);
    assert_eq!(ms_a.get_node(1).end(), 5);
    assert_eq!(ms_a.get(1).begin(), 12);
    assert_eq!(ms_a.get(1).end(), 15);
}
