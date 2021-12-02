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

use std::ops::Deref;
use sudachi::prelude::*;

mod common;
use crate::common::TestTokenizer;

#[test]
fn empty_morpheme_list() {
    let tok = TestTokenizer::new();
    let empty = MorphemeList::empty(tok.dict());

    assert_eq!("", empty.surface().deref());
    assert_eq!(0, empty.len());
}

#[test]
fn morpheme_attributes() {
    let tok = TestTokenizer::new();
    let ms = tok.tokenize("京都", Mode::C);

    assert_eq!(0, ms.get(0).begin());
    assert_eq!(6, ms.get(0).end());
    assert_eq!("京都", ms.get(0).surface().deref());

    assert_eq!(
        ["名詞", "固有名詞", "地名", "一般", "*", "*"],
        ms.get(0).part_of_speech()
    );
    assert_eq!(3, ms.get(0).part_of_speech_id());

    assert_eq!("京都", ms.get(0).dictionary_form());
    assert_eq!("京都", ms.get(0).normalized_form());
    assert_eq!("キョウト", ms.get(0).reading_form());

    assert_eq!(false, ms.get(0).is_oov());

    assert_eq!(3, ms.get(0).word_id().word());
    assert_eq!(0, ms.get(0).dictionary_id());
    assert_eq!([1, 5], ms.get(0).synonym_group_ids());
}

#[test]
fn split_morpheme() {
    let tok = TestTokenizer::new();
    let ms = tok.tokenize("京都東京都", Mode::C);
    assert_eq!(2, ms.len());
    assert_eq!("京都", ms.get(0).surface().deref());
    assert_eq!("東京都", ms.get(1).surface().deref());

    #[allow(deprecated)]
    let ms = ms.get(1).split(Mode::A).expect("failed to split morpheme");
    assert_eq!(2, ms.len());
    assert_eq!("東京", ms.get(0).surface().deref());
    assert_eq!(6, ms.get(0).begin()); // keep index for the whole input text
    assert_eq!(12, ms.get(0).end());
    assert_eq!("都", ms.get(1).surface().deref());
}
