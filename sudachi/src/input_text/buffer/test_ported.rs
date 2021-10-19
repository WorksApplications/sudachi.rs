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

use super::*;
use crate::test::cat_grammar;

#[test]
fn get_original_text() {
    let grammar = cat_grammar();
    let mut builder = InputBuffer::new();
    builder.reset().push_str("âｂC1あ234漢字𡈽アｺﾞ");
    builder.start_build().expect("succeeds");
    assert_eq!("âｂC1あ234漢字𡈽アｺﾞ", builder.original);
    assert_eq!("âｂC1あ234漢字𡈽アｺﾞ", builder.modified);

    builder.build(&grammar).expect("succeeds");
    assert_eq!("âｂC1あ234漢字𡈽アｺﾞ", builder.original());
    assert_eq!("âｂC1あ234漢字𡈽アｺﾞ", builder.current());
}

// skip get_original_index, our editing tests are better

#[test]
fn get_char_category_types() {
    let grammar = cat_grammar();
    let mut builder = InputBuffer::from("âｂC1あ234漢字𡈽アｺﾞ");
    builder.build(&grammar).expect("succeeds");
    let input = &builder;
    assert!(input.cat_at_char(0).contains(CategoryType::ALPHA)); // â
    assert!(input.cat_at_char(1).contains(CategoryType::ALPHA)); // ｂ
    assert!(input.cat_at_char(2).contains(CategoryType::ALPHA)); // C
    assert!(input.cat_at_char(3).contains(CategoryType::NUMERIC)); // 1
    assert!(input.cat_at_char(4).contains(CategoryType::HIRAGANA)); // あ
    assert!(input.cat_at_char(5).contains(CategoryType::NUMERIC)); // 2
    assert!(input.cat_at_char(6).contains(CategoryType::NUMERIC)); // 3
    assert!(input.cat_at_char(7).contains(CategoryType::NUMERIC)); // 4
    assert!(input.cat_at_char(8).contains(CategoryType::KANJI)); // 漢
    assert!(input.cat_at_char(9).contains(CategoryType::KANJI)); // 字
    assert!(input.cat_at_char(10).contains(CategoryType::DEFAULT)); // 𡈽
    assert!(input.cat_at_char(11).contains(CategoryType::KATAKANA)); // ア
    assert!(input.cat_at_char(12).contains(CategoryType::KATAKANA)); // ｺ
    assert!(input.cat_at_char(13).contains(CategoryType::KATAKANA)); // ﾞ
}

#[test]
fn get_char_category_continuous_length() {
    let grammar = cat_grammar();
    let mut input = InputBuffer::from("âｂC1あ234漢字𡈽アｺﾞ");
    input.build(&grammar).expect("works");
    assert_eq!(3, input.cat_continuous_len(0)); // â
    assert_eq!(2, input.cat_continuous_len(1)); // ｂ
    assert_eq!(1, input.cat_continuous_len(2)); // C
    assert_eq!(1, input.cat_continuous_len(3)); // 1
    assert_eq!(1, input.cat_continuous_len(4)); // あ
    assert_eq!(3, input.cat_continuous_len(5)); // 2
    assert_eq!(2, input.cat_continuous_len(6)); // 3
    assert_eq!(1, input.cat_continuous_len(7)); // 4
    assert_eq!(2, input.cat_continuous_len(8)); // 漢
    assert_eq!(1, input.cat_continuous_len(9)); // 字
    assert_eq!(1, input.cat_continuous_len(10)); // 𡈽
    assert_eq!(3, input.cat_continuous_len(11)); // ア
    assert_eq!(2, input.cat_continuous_len(12)); // ｺ
    assert_eq!(1, input.cat_continuous_len(13)); // ﾞ
}

#[test]
fn range_cat() {
    let grammar = cat_grammar();
    let mut input = InputBuffer::from("âｂC1あ234漢字𡈽アｺﾞ");
    input.build(&grammar).expect("works");
    assert_eq!(input.cat_of_range(0..3), CategoryType::ALPHA); // âｂC
    assert_eq!(input.cat_of_range(3..5), CategoryType::empty()); // 1あ
    assert_eq!(input.cat_of_range(8..10), CategoryType::KANJI); // 漢字
}

// replace_* tests -> new edit tests are better and easier to figure about

#[test]
fn can_bow() {
    let grammar = cat_grammar();
    let mut input = InputBuffer::from("âｂC1あ234漢字𡈽アｺﾞ");
    input.build(&grammar).expect("works");
    assert!(input.can_bow(0)); // â
    assert!(!input.can_bow(1));
    assert!(!input.can_bow(2)); // ｂ
    assert!(!input.can_bow(3));
    assert!(!input.can_bow(4));
    assert!(!input.can_bow(5)); // C
    assert!(input.can_bow(6)); // 1
    assert!(input.can_bow(7)); // あ

    assert!(input.can_bow(19)); // 𡈽
    assert!(!input.can_bow(20));
    assert!(!input.can_bow(21));
    assert!(!input.can_bow(22));
    assert!(input.can_bow(23)); // ア
}

#[test]
fn get_word_candidate_length() {
    let grammar = cat_grammar();
    let mut input = InputBuffer::from("âｂC1あ234漢字𡈽アｺﾞ");
    input.build(&grammar).expect("works");
    assert_eq!(3, input.get_word_candidate_length(0)); // â
    assert_eq!(1, input.get_word_candidate_length(4)); // 1
    assert_eq!(1, input.get_word_candidate_length(10)); // 𡈽
    assert_eq!(1, input.get_word_candidate_length(13)); // ﾞ
}
