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

extern crate lazy_static;

mod common;
use common::LEXICON;
use sudachi::dic::lexicon::LexiconEntry;
use sudachi::dic::subset::InfoSubset;
use sudachi::dic::word_id::WordId;

#[test]
fn lookup() {
    let res: Vec<LexiconEntry> = LEXICON.lookup("東京都".as_bytes(), 0).collect();
    assert_eq!(3, res.len());
    assert_eq!(LexiconEntry::new(WordId::from_raw(4), 3), res[0]); // 東
    assert_eq!(LexiconEntry::new(WordId::from_raw(5), 6), res[1]); // 東京
    assert_eq!(LexiconEntry::new(WordId::from_raw(6), 9), res[2]); // 東京都

    let res: Vec<LexiconEntry> = LEXICON.lookup("東京都に".as_bytes(), 9).collect();
    assert_eq!(2, res.len());
    assert_eq!(LexiconEntry::new(WordId::from_raw(1), 12), res[0]); // に(接続助詞)
    assert_eq!(LexiconEntry::new(WordId::from_raw(2), 12), res[1]); // に(格助詞)

    let res: Vec<LexiconEntry> = LEXICON.lookup("あれ".as_bytes(), 0).collect();
    assert_eq!(0, res.len());
}

#[test]
fn parameters() {
    // た
    assert_eq!((1, 1, 8729), LEXICON.get_word_param(0));

    // 東京都
    assert_eq!((6, 8, 5320), LEXICON.get_word_param(6));

    // 都
    assert_eq!((8, 8, 2914), LEXICON.get_word_param(9));
}

#[test]
fn word_info() {
    // た
    let wi = LEXICON
        .get_word_info(0, InfoSubset::all())
        .expect("failed to get word_info");
    assert_eq!("た", wi.surface());
    assert_eq!(3, wi.head_word_length());
    assert_eq!(0, wi.pos_id());
    assert_eq!("た", wi.normalized_form());
    assert_eq!(-1, wi.dictionary_form_word_id());
    assert_eq!("た", wi.dictionary_form());
    assert_eq!("タ", wi.reading_form());
    assert!(wi.a_unit_split().is_empty());
    assert!(wi.b_unit_split().is_empty());
    assert!(wi.word_structure().is_empty());

    // 東京都
    let wi = LEXICON
        .get_word_info(6, InfoSubset::all())
        .expect("failed to get word_info");
    assert_eq!("東京都", wi.surface());
    assert_eq!(
        [WordId::from_raw(5), WordId::from_raw(9)],
        wi.a_unit_split()
    );
    assert!(wi.b_unit_split().is_empty());
    assert_eq!(
        [WordId::from_raw(5), WordId::from_raw(9)],
        wi.word_structure()
    );
    assert!(wi.synonym_group_ids().is_empty());

    // 行っ
    let wi = LEXICON
        .get_word_info(8, InfoSubset::all())
        .expect("failed to get word_info");
    assert_eq!("行っ", wi.surface());
    assert_eq!("行く", wi.normalized_form());
    assert_eq!(7, wi.dictionary_form_word_id());
    assert_eq!("行く", wi.dictionary_form());
}

#[test]
fn word_info_with_longword() {
    // 0123456789 * 30
    let wi = LEXICON
        .get_word_info(36, InfoSubset::all())
        .expect("failed to get word_info");
    assert_eq!(300, wi.surface().chars().count());
    assert_eq!(300, wi.head_word_length());
    assert_eq!(300, wi.normalized_form().chars().count());
    assert_eq!(-1, wi.dictionary_form_word_id());
    assert_eq!(300, wi.dictionary_form().chars().count());
    assert_eq!(570, wi.reading_form().chars().count());
}

#[test]
fn size() {
    assert_eq!(39, LEXICON.size())
}
