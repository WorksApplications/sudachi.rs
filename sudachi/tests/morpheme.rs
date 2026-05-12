/*
 *  Copyright (c) 2021-2026 Works Applications Co., Ltd.
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
use sudachi::dic::subset::InfoSubset;
use sudachi::prelude::*;

mod common;
use crate::common::{TestTokenizer, LEXICON_SET};

#[test]
fn empty_morpheme_list() {
    let tok = TestTokenizer::new();
    let empty = MorphemeList::empty(tok.dict());

    assert_eq!("", empty.surface().deref());
    assert_eq!(0, empty.len());
}

#[test]
fn reset_with_word_id_uses_exact_homograph_entry() {
    let tok = TestTokenizer::new();
    let word_ids = LEXICON_SET.system_word_ids_in_order();
    let mut ms = MorphemeList::empty(tok.dict());

    // These are the first and last "x" homographs in tests/resources/lex.csv;
    // they share headword "X" but have different POS metadata.
    let first_x = *word_ids.get(40).unwrap();
    let place_name_x = *word_ids.get(45).unwrap();

    ms.reset_with_word_id(first_x, InfoSubset::POS_ID)
        .expect("failed to materialize first x entry");
    assert_eq!(1, ms.len());
    assert_eq!(first_x, ms.get(0).word_id());
    assert_eq!("X", ms.get(0).surface().deref());
    assert_eq!(0, ms.get(0).begin());
    assert_eq!(1, ms.get(0).end());
    assert_eq!(
        ["補助記号", "一般", "*", "*", "*", "*"],
        ms.get(0).part_of_speech()
    );

    ms.reset_with_word_id(place_name_x, InfoSubset::all())
        .expect("failed to materialize place-name x entry");
    assert_eq!(1, ms.len());
    assert_eq!(place_name_x, ms.get(0).word_id());
    assert_eq!("X", ms.get(0).surface().deref());
    assert_eq!(
        ["名詞", "固有名詞", "地名", "一般", "*", "*"],
        ms.get(0).part_of_speech()
    );
}

#[test]
fn dictionary_form_morpheme_returns_standalone_entry() {
    let tok = TestTokenizer::new();
    let ms = tok.tokenize("行っ", Mode::C);
    let m = ms.get(0);

    let df = m
        .dictionary_form_morpheme()
        .expect("failed to resolve dictionary form morpheme");

    assert!(matches!(df, MorphemeRef::Single(_)));
    assert_eq!("行く", df.surface().deref());
    assert_eq!(m.dictionary_form(), df.surface().deref());
    assert_eq!("行く", df.dictionary_form());
    assert_eq!("イク", df.reading_form());
    assert_eq!(0, df.begin());
    assert_eq!("行く".len(), df.end());
    assert_eq!(0, df.begin_c());
    assert_eq!("行く".chars().count(), df.end_c());
    assert!(!df.is_oov());
}

#[test]
fn normalized_form_morpheme_returns_standalone_entry() {
    let tok = TestTokenizer::new();
    let ms = tok.tokenize("いっ", Mode::C);
    let m = ms.get(0);

    let nf = m
        .normalized_form_morpheme()
        .expect("failed to resolve normalized form morpheme");

    assert!(matches!(nf, MorphemeRef::Single(_)));
    assert_eq!("行く", nf.surface().deref());
    assert_eq!(m.normalized_form(), nf.surface().deref());
    assert_eq!("行く", nf.normalized_form());
    assert_eq!("イク", nf.reading_form());
    assert_eq!(0, nf.begin());
    assert_eq!("行く".len(), nf.end());
    assert_eq!(0, nf.begin_c());
    assert_eq!("行く".chars().count(), nf.end_c());
    assert!(!nf.is_oov());
}

#[test]
fn form_morpheme_returns_self_equivalent_for_same_entry_and_oov() {
    let tok = TestTokenizer::new();
    let ms = tok.tokenize("東京", Mode::C);
    let m = ms.get(0);

    let df = m
        .dictionary_form_morpheme()
        .expect("failed to resolve dictionary form morpheme");
    let nf = m
        .normalized_form_morpheme()
        .expect("failed to resolve normalized form morpheme");

    assert!(matches!(df, MorphemeRef::ListItem(_)));
    assert!(matches!(nf, MorphemeRef::ListItem(_)));
    assert_eq!(m.word_id(), df.word_id());
    assert_eq!(m.word_id(), nf.word_id());
    assert_eq!(m.surface().deref(), df.surface().deref());
    assert_eq!(m.surface().deref(), nf.surface().deref());

    let ms = tok.tokenize("xyzzy123不在語", Mode::C);
    let oov = ms
        .iter()
        .find(|m| m.is_oov())
        .expect("expected at least one OOV morpheme");
    let df = oov
        .dictionary_form_morpheme()
        .expect("failed to resolve OOV dictionary form morpheme");
    let nf = oov
        .normalized_form_morpheme()
        .expect("failed to resolve OOV normalized form morpheme");

    assert!(matches!(df, MorphemeRef::ListItem(_)));
    assert!(matches!(nf, MorphemeRef::ListItem(_)));
    assert_eq!(oov.word_id(), df.word_id());
    assert_eq!(oov.word_id(), nf.word_id());
    assert_eq!(oov.surface().deref(), df.surface().deref());
    assert_eq!(oov.surface().deref(), nf.surface().deref());
    assert!(df.is_oov());
    assert!(nf.is_oov());
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

    assert!(!ms.get(0).is_oov());

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
