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

#[macro_use]
extern crate lazy_static;
extern crate sudachi;

use std::rc::Rc;

use common::TEST_CONFIG;
use sudachi::dic::dictionary::JapaneseDictionary;
use sudachi::prelude::Morpheme;
use sudachi::stateless_tokeniser::StatelessTokenizer;
use sudachi::tokenizer::{Mode, Tokenize};

mod common;

struct TestTokenizer {
    tok: StatelessTokenizer<Rc<JapaneseDictionary>>,
}

impl TestTokenizer {
    fn new() -> TestTokenizer {
        let dict = JapaneseDictionary::from_cfg(&TEST_CONFIG).expect("failed to make dictionary");
        let rcdict = Rc::new(dict);
        let tok = StatelessTokenizer::new(rcdict.clone());
        return TestTokenizer { tok };
    }

    fn tokenize(&self, data: &str, mode: Mode) -> Vec<Morpheme> {
        let result = self.tok.tokenize(data, mode, false);
        result.expect("tokenization failed")
    }
}

#[test]
fn tokenize_small_katakana_only() {
    let tok = TestTokenizer::new();
    let ms = tok.tokenize("ァ", Mode::C);
    assert_eq!(1, ms.len());
}

#[test]
fn get_word_id() {
    let tok = TestTokenizer::new();
    let ms = tok.tokenize("京都", Mode::C);
    assert_eq!(1, ms.len());
    let pos = ms[0].pos().expect("failed to get pos");
    assert_eq!(["名詞", "固有名詞", "地名", "一般", "*", "*"], &pos[..]);

    // we do not have word_id field in Morpheme and skip testing.
    let ms = tok.tokenize("ぴらる", Mode::C);
    assert_eq!(1, ms.len());
    let pos = ms[0].pos().expect("failed to get pos");
    assert_eq!(["名詞", "普通名詞", "一般", "*", "*", "*"], &pos[..]);
}

#[test]
fn get_dictionary_id() {
    let tok = TestTokenizer::new();
    let ms = tok.tokenize("京都", Mode::C);
    assert_eq!(1, ms.len());
    assert_eq!(0, ms[0].dictionary_id);

    let ms = tok.tokenize("ぴらる", Mode::C);
    assert_eq!(1, ms.len());
    assert_eq!(1, ms[0].dictionary_id);

    let ms = tok.tokenize("京", Mode::C);
    assert_eq!(1, ms.len());
    assert!(ms[0].dictionary_id < 0);
}

#[test]
fn get_synonym_group_id() {
    let tok = TestTokenizer::new();
    let ms = tok.tokenize("京都", Mode::C);
    assert_eq!(1, ms.len());
    assert_eq!([1, 5], ms[0].word_info.synonym_group_ids.as_slice());

    let ms = tok.tokenize("ぴらる", Mode::C);
    assert_eq!(1, ms.len());
    assert!(ms[0].word_info.synonym_group_ids.is_empty());

    let ms = tok.tokenize("東京府", Mode::C);
    assert_eq!(1, ms.len());
    assert_eq!([1, 3], ms[0].word_info.synonym_group_ids.as_slice());
}

#[test]
fn tokenize_kanji_alphabet_word() {
    let tok = TestTokenizer::new();
    assert_eq!(1, tok.tokenize("特a", Mode::C).len());
    assert_eq!(1, tok.tokenize("ab", Mode::C).len());
    assert_eq!(2, tok.tokenize("特ab", Mode::C).len());
}

#[test]
fn tokenize_with_dots() {
    let tok = TestTokenizer::new();
    let ms = tok.tokenize("京都…", Mode::C);
    assert_eq!(4, ms.len());
    assert_eq!("…", ms[1].surface());
    assert_eq!(".", ms[1].normalized_form());
    assert_eq!("", ms[2].surface());
    assert_eq!(".", ms[2].normalized_form());
    assert_eq!("", ms[3].surface());
    assert_eq!(".", ms[3].normalized_form());
}

#[test]
fn tokenizer_morpheme_split() {
    let tok = TestTokenizer::new();
    let ms = tok.tokenize("東京都", Mode::C);
    assert_eq!(1, ms.len());
    assert_eq!("東京都", ms[0].surface());

    let ms = tok.tokenize("東京都", Mode::A);
    assert_eq!(2, ms.len());
    assert_eq!("東京", ms[0].surface());
    assert_eq!("都", ms[1].surface());
}
