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
    assert!(input.cat_at_byte(0).contains(CategoryType::ALPHA));
    assert!(input.cat_at_byte(2).contains(CategoryType::ALPHA));
    assert!(input.cat_at_byte(5).contains(CategoryType::ALPHA));
    assert!(input.cat_at_byte(6).contains(CategoryType::NUMERIC));
    assert!(input.cat_at_byte(7).contains(CategoryType::HIRAGANA));
    assert!(input.cat_at_byte(9).contains(CategoryType::HIRAGANA));
    assert!(input.cat_at_byte(10).contains(CategoryType::NUMERIC));
    assert!(input.cat_at_byte(13).contains(CategoryType::KANJI));
    assert!(input.cat_at_byte(18).contains(CategoryType::KANJI));
    assert!(input.cat_at_byte(19).contains(CategoryType::DEFAULT));
    assert!(input.cat_at_byte(22).contains(CategoryType::DEFAULT));
    assert!(input.cat_at_byte(23).contains(CategoryType::KATAKANA));
    assert!(input.cat_at_byte(26).contains(CategoryType::KATAKANA));
    assert!(input.cat_at_byte(31).contains(CategoryType::KATAKANA));
}

#[test]
fn get_char_category_continuous_length() {
    let grammar = cat_grammar();
    let mut input = InputBuffer::from("âｂC1あ234漢字𡈽アｺﾞ");
    input.build(&grammar).expect("works");
    assert_eq!(6, input.cat_continuous_len(0));
    assert_eq!(5, input.cat_continuous_len(1));
    assert_eq!(4, input.cat_continuous_len(2));
    assert_eq!(1, input.cat_continuous_len(5));
    assert_eq!(1, input.cat_continuous_len(6));
    assert_eq!(3, input.cat_continuous_len(7));
    assert_eq!(3, input.cat_continuous_len(10));
    assert_eq!(2, input.cat_continuous_len(11));
    assert_eq!(1, input.cat_continuous_len(12));
    assert_eq!(4, input.cat_continuous_len(19));
    assert_eq!(1, input.cat_continuous_len(22));
    assert_eq!(9, input.cat_continuous_len(23));
    assert_eq!(6, input.cat_continuous_len(26));
    assert_eq!(1, input.cat_continuous_len(31));
}
