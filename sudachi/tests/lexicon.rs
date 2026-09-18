/*
 * Copyright (c) 2021-2026 Works Applications Co., Ltd.
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
use common::{LEXICON, LEXICON_SET};
use sudachi::dic::lexicon::LexiconEntry;

#[test]
fn lookup() {
    let wids = LEXICON_SET.system_word_ids_in_order();

    let res: Vec<LexiconEntry> = LEXICON.lookup("東京都".as_bytes(), 0).collect();
    assert_eq!(3, res.len());
    assert_eq!(LexiconEntry::new(*wids.get(4).unwrap(), 3), res[0]); // 東
    assert_eq!(LexiconEntry::new(*wids.get(5).unwrap(), 6), res[1]); // 東京
    assert_eq!(LexiconEntry::new(*wids.get(6).unwrap(), 9), res[2]); // 東京都

    let res: Vec<LexiconEntry> = LEXICON.lookup("東京都に".as_bytes(), 9).collect();
    assert_eq!(2, res.len());
    assert_eq!(LexiconEntry::new(*wids.get(1).unwrap(), 12), res[0]); // に(接続助詞)
    assert_eq!(LexiconEntry::new(*wids.get(2).unwrap(), 12), res[1]); // に(格助詞)

    let res: Vec<LexiconEntry> = LEXICON.lookup("あれ".as_bytes(), 0).collect();
    assert_eq!(0, res.len());
}

#[test]
fn parameters() {
    let eids = LEXICON.entry_ids_in_order();

    // た
    assert_eq!((1, 1, 8729), LEXICON.get_word_param(*eids.get(0).unwrap()));

    // 東京都
    assert_eq!((6, 8, 5320), LEXICON.get_word_param(*eids.get(6).unwrap()));

    // 都
    assert_eq!((8, 8, 2914), LEXICON.get_word_param(*eids.get(9).unwrap()));
}

#[test]
fn word_info() {
    let wids = LEXICON_SET.system_word_ids_in_order();

    // た
    let wi = LEXICON_SET
        .get_word_info(*wids.get(0).unwrap())
        .expect("failed to get word_info");
    assert_eq!("た", wi.headword(&LEXICON_SET));
    assert_eq!(3, wi.index_form_length());
    assert_eq!(0, wi.pos_id());
    assert_eq!("た", wi.normalized_form(&LEXICON_SET));
    assert_eq!(
        *wids.get(0).unwrap(),
        wi.borrow_data().dictionary_form_word_id()
    );
    assert_eq!("た", wi.dictionary_form(&LEXICON_SET));
    assert_eq!("タ", wi.reading_form(&LEXICON_SET));
    assert!(wi.a_unit_split().is_empty());
    assert!(wi.b_unit_split().is_empty());
    assert!(wi.word_structure().is_empty());

    // 東京都
    let wi = LEXICON_SET
        .get_word_info(*wids.get(6).unwrap())
        .expect("failed to get word_info");
    assert_eq!("東京都", wi.headword(&LEXICON_SET));
    assert_eq!(
        [*wids.get(5).unwrap(), *wids.get(9).unwrap()],
        wi.a_unit_split()
    );
    assert!(wi.b_unit_split().is_empty());
    assert_eq!(
        [*wids.get(5).unwrap(), *wids.get(9).unwrap()],
        wi.word_structure()
    );
    assert!(wi.synonym_group_ids().is_empty());

    // 行っ
    let wi = LEXICON_SET
        .get_word_info(*wids.get(8).unwrap())
        .expect("failed to get word_info");
    assert_eq!("行っ", wi.headword(&LEXICON_SET));
    assert_eq!("行く", wi.normalized_form(&LEXICON_SET));
    assert_eq!(
        *wids.get(7).unwrap(),
        wi.borrow_data().dictionary_form_word_id()
    );
    assert_eq!("行く", wi.dictionary_form(&LEXICON_SET));
}

#[test]
fn word_info_with_longword() {
    let wids = LEXICON_SET.system_word_ids_in_order();

    // 0123456789 * 30
    let wi = LEXICON_SET
        .get_word_info(*wids.get(36).unwrap())
        .expect("failed to get word_info");
    assert_eq!(300, wi.headword(&LEXICON_SET).chars().count());
    assert_eq!(300, wi.index_form_length());
    assert_eq!(300, wi.normalized_form(&LEXICON_SET).chars().count());
    assert_eq!(
        *wids.get(36).unwrap(),
        wi.borrow_data().dictionary_form_word_id()
    );
    assert_eq!(300, wi.dictionary_form(&LEXICON_SET).chars().count());
    assert_eq!(570, wi.reading_form(&LEXICON_SET).chars().count());
}

#[test]
fn size() {
    assert_eq!(46, LEXICON.size())
}
