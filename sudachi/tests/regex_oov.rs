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

use crate::common::{TestStatefulTokenizer, LEX_CSV, USER1_CSV, USER2_CSV};
use std::ops::Deref;

mod common;

const REGEX_CONFIG: &'static [u8] = include_bytes!("resources/sudachi.regex.json");

#[test]
fn no_other_words() {
    let mut tok = TestStatefulTokenizer::builder(LEX_CSV)
        .config(REGEX_CONFIG)
        .build();
    let tokens = tok.tokenize("XAG-2F");
    assert_eq!(1, tokens.len());
    assert_eq!("XAG-2F", tokens.get(0).surface().deref());
    assert_eq!("REGEX", tokens.get(0).part_of_speech()[2])
}

#[test]
fn has_other_words() {
    let mut tok = TestStatefulTokenizer::builder(LEX_CSV)
        .config(REGEX_CONFIG)
        .build();
    let tokens = tok.tokenize("京都XAG-2F東京");
    assert_eq!(3, tokens.len());
    assert_eq!("XAG-2F", tokens.get(1).surface().deref());
}

#[test]
fn has_other_conflicting_words() {
    let mut tok = TestStatefulTokenizer::builder(LEX_CSV)
        .config(REGEX_CONFIG)
        .build();
    let tokens = tok.tokenize("２つXＡＧ-2F");
    assert_eq!(3, tokens.len());
    assert_eq!("XＡＧ-2F", tokens.get(2).surface().deref());
}

#[test]
fn digits_in_middle() {
    let mut tok = TestStatefulTokenizer::builder(LEX_CSV)
        .config(REGEX_CONFIG)
        .build();
    let tokens = tok.tokenize("AVX-512F");
    assert_eq!(1, tokens.len());
    assert_eq!("AVX-512F", tokens.get(0).surface().deref());
}

#[test]
fn very_long_word_not_added() {
    let mut tok = TestStatefulTokenizer::builder(LEX_CSV)
        .config(REGEX_CONFIG)
        .build();
    let data = "0123456789".repeat(30);
    let tokens = tok.tokenize(&data);
    assert_eq!(1, tokens.len());
    assert_eq!(&data, tokens.get(0).surface().deref());
    assert_eq!("数詞", tokens.get(0).part_of_speech()[1])
}

#[test]
fn user_dictionaries_have_correct_pos() {
    let mut tok = TestStatefulTokenizer::builder(LEX_CSV)
        .config(REGEX_CONFIG)
        .user(USER1_CSV)
        .user(USER2_CSV)
        .build();
    let tokens = tok.tokenize("すだちASDF12かぼす");
    assert_eq!(3, tokens.len());
    assert_eq!("すだち", tokens.get(0).surface().deref());
    assert_eq!("スダチ", tokens.get(0).part_of_speech()[5]);
    assert_eq!("ASDF12", tokens.get(1).surface().deref());
    assert_eq!("REGEX", tokens.get(1).part_of_speech()[5]);
    assert_eq!("かぼす", tokens.get(2).surface().deref());
    assert_eq!("カボス", tokens.get(2).part_of_speech()[5]);
}
