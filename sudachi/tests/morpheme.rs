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
use sudachi::dic::word_id::WordId;
use sudachi::prelude::*;

mod common;
use crate::common::{TestStatefulTokenizer, TestTokenizer, LEXICON_SET};

const SINGLE_SPLIT_DIC: &str = "\
index_form,left_id,right_id,cost,headword,pos1,pos2,pos3,pos4,pos5,pos6,reading_form,normalized_form,dictionary_form,split_a,split_b,split_c,word_structure,synonym_groups
ab,6,6,1000,AB,名詞,普通名詞,一般,*,*,*,AB,,,\"A,1,A\",,,,
a,-1,6,1000,A,名詞,数詞,*,*,*,*,A,,,,,,,
";

fn find_system_word_id(tok: &TestTokenizer, headword: &str, pos: [&str; 6]) -> WordId {
    LEXICON_SET
        .system_word_ids_in_order()
        .into_iter()
        .find(|&word_id| {
            let word_info = LEXICON_SET
                .get_word_info_subset(word_id, InfoSubset::HEADWORD | InfoSubset::POS_ID)
                .expect("failed to load word info");
            let actual_pos = tok.dict().grammar().pos_components(word_info.pos_id());
            word_info.headword(&*LEXICON_SET) == headword
                && actual_pos
                    .iter()
                    .map(String::as_str)
                    .eq(pos.iter().copied())
        })
        .expect("expected test dictionary entry")
}

fn x_homograph_word_ids(tok: &TestTokenizer) -> (WordId, WordId) {
    let first_x = find_system_word_id(tok, "X", ["補助記号", "一般", "*", "*", "*", "*"]);
    let place_name_x =
        find_system_word_id(tok, "X", ["名詞", "固有名詞", "地名", "一般", "*", "*"]);
    (first_x, place_name_x)
}

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
    let mut ms = MorphemeList::empty(tok.dict());

    // These are "x" homographs in tests/resources/lex.csv;
    // they share headword "X" but have different POS metadata.
    let (first_x, place_name_x) = x_homograph_word_ids(&tok);

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
    assert_eq!("", ms.get(0).user_data());

    ms.reset_with_word_id(place_name_x, InfoSubset::all())
        .expect("failed to materialize place-name x entry");
    assert_eq!(1, ms.len());
    assert_eq!(place_name_x, ms.get(0).word_id());
    assert_eq!("X", ms.get(0).surface().deref());
    assert_eq!(
        ["名詞", "固有名詞", "地名", "一般", "*", "*"],
        ms.get(0).part_of_speech()
    );
    assert_eq!("", ms.get(0).user_data());
}

#[test]
fn single_morpheme_from_word_id_uses_exact_homograph_entry() {
    let tok = TestTokenizer::new();

    // These are "x" homographs in tests/resources/lex.csv;
    // they share headword "X" but have different POS metadata.
    let (first_x, place_name_x) = x_homograph_word_ids(&tok);

    let first = SingleMorpheme::from_word_id(tok.dict(), first_x, InfoSubset::all())
        .expect("failed to materialize first x entry");
    let place = SingleMorpheme::from_word_id(tok.dict(), place_name_x, InfoSubset::all())
        .expect("failed to materialize place-name x entry");

    assert_eq!(first_x, first.word_id());
    assert_eq!(place_name_x, place.word_id());
    assert_eq!("X", first.surface());
    assert_eq!("X", place.surface());
    assert_eq!(
        ["補助記号", "一般", "*", "*", "*", "*"],
        first.part_of_speech()
    );
    assert_eq!(
        ["名詞", "固有名詞", "地名", "一般", "*", "*"],
        place.part_of_speech()
    );
    assert_eq!("", first.user_data());
    assert_eq!("", place.user_data());
}

#[test]
fn word_id_materialization_rejects_invalid_oov_and_special_ids() {
    let tok = TestTokenizer::new();
    let mut ms = MorphemeList::empty(tok.dict());

    for word_id in [WordId::INVALID, WordId::oov(0), WordId::BOS, WordId::EOS] {
        assert!(matches!(
            SingleMorpheme::from_word_id(tok.dict(), word_id, InfoSubset::all()),
            Err(SudachiError::InvalidWordId(err_word_id)) if err_word_id == word_id
        ));
        assert!(matches!(
            ms.reset_with_word_id(word_id, InfoSubset::all()),
            Err(SudachiError::InvalidWordId(err_word_id)) if err_word_id == word_id
        ));
    }
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
    assert_eq!("", df.user_data());
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
    assert_eq!("", nf.user_data());
}

#[test]
fn single_morpheme_split_materializes_standalone_entries() {
    let tok = TestTokenizer::new();
    let ms = tok.tokenize("東京都", Mode::C);
    let morpheme = SingleMorpheme::from_word_id(tok.dict(), ms.get(0).word_id(), InfoSubset::all())
        .expect("failed to materialize standalone morpheme");

    let splits = morpheme
        .split(Mode::A)
        .expect("failed to split standalone morpheme");

    assert_eq!(2, splits.len());
    assert_eq!("東京", splits[0].surface());
    assert_eq!("都", splits[1].surface());
    assert_eq!(0, splits[0].begin());
    assert_eq!("東京".len(), splits[0].end());
    assert_eq!("東京".len(), splits[1].begin());
    assert_eq!("東京都".len(), splits[1].end());
    assert_eq!(0, splits[0].begin_c());
    assert_eq!("東京".chars().count(), splits[0].end_c());
    assert_eq!("東京".chars().count(), splits[1].begin_c());
    assert_eq!("東京都".chars().count(), splits[1].end_c());
    assert_eq!("", splits[0].user_data());
    assert_eq!("", splits[1].user_data());
}

#[test]
fn single_morpheme_split_loads_split_subset_on_demand() {
    let tok = TestTokenizer::new();
    let ms = tok.tokenize("東京都", Mode::C);
    let word_id = ms.get(0).word_id();

    let surface_only = SingleMorpheme::from_word_id(tok.dict(), word_id, InfoSubset::HEADWORD)
        .expect("failed to materialize standalone morpheme");

    let splits = surface_only
        .split(Mode::A)
        .expect("failed to split standalone morpheme");
    assert_eq!(2, splits.len());
    assert_eq!("東京", splits[0].surface());
    assert_eq!("都", splits[1].surface());

    let with_splits = SingleMorpheme::from_word_id(
        tok.dict(),
        word_id,
        InfoSubset::HEADWORD | InfoSubset::SPLIT_A,
    )
    .expect("failed to materialize standalone morpheme with splits");

    let splits = with_splits
        .split(Mode::A)
        .expect("failed to split standalone morpheme");
    assert_eq!(2, splits.len());
    assert_eq!("東京", splits[0].surface());
    assert_eq!("都", splits[1].surface());
}

#[test]
fn single_morpheme_split_single_replacement_preserves_span() {
    let mut tok = TestStatefulTokenizer::builder(SINGLE_SPLIT_DIC.as_bytes())
        .mode(Mode::C)
        .build();
    let word_id = {
        let ms = tok.tokenize("ＡＢ");
        assert_eq!(1, ms.len());
        assert_eq!("ＡＢ", ms.get(0).surface().deref());
        ms.get(0).word_id()
    };
    let morpheme = SingleMorpheme::from_word_id(
        tok.dict(),
        word_id,
        InfoSubset::HEADWORD | InfoSubset::SPLIT_A,
    )
    .expect("failed to materialize standalone morpheme");

    let splits = morpheme
        .split(Mode::A)
        .expect("failed to split standalone morpheme");

    assert_eq!(1, splits.len());
    assert_ne!(morpheme.word_id(), splits[0].word_id());
    assert_eq!("A", splits[0].surface());
    assert_eq!(morpheme.begin(), splits[0].begin());
    assert_eq!(morpheme.end(), splits[0].end());
    assert_eq!(morpheme.begin_c(), splits[0].begin_c());
    assert_eq!(morpheme.end_c(), splits[0].end_c());
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
