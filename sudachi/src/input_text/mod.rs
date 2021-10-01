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

pub mod utf8_input_text;
pub mod utf8_input_text_builder;

pub use utf8_input_text::Utf8InputText;
pub use utf8_input_text_builder::Utf8InputTextBuilder;

#[cfg(test)]
mod tests {
    use super::utf8_input_text_builder::Utf8InputTextBuilder;
    use crate::dic::category_type::CategoryType;
    use crate::dic::character_category::CharacterCategory;
    use crate::dic::grammar::Grammar;
    use std::path::PathBuf;

    const CHAR_CATEGORY_FILE_PATH: &str = "tests/resources/char.def";
    const TEXT: &str = "âｂC1あ234漢字𡈽アｺﾞ";
    // const BYTES: &[u8] = &[
    //     0xC3u8, 0xA2u8, 0xEFu8, 0xBDu8, 0x82u8, 0x43u8, 0x31u8, 0xE3u8, 0x81u8, 0x82u8, 0x32u8,
    //     0x33u8, 0x34u8, 0xE6u8, 0xBCu8, 0xA2u8, 0xE5u8, 0xADu8, 0x97u8, 0xF0u8, 0xA1u8, 0x88u8,
    //     0xBDu8, 0xE3u8, 0x82u8, 0xA2u8, 0xEFu8, 0xBDu8, 0xBAu8, 0xEFu8, 0xBEu8, 0x9Eu8,
    // ];

    fn build_mock_bytes() -> Vec<u8> {
        let mut buf = Vec::new();
        // set 0 for all of pos size, left and right id size
        buf.extend(&(0 as i16).to_le_bytes());
        buf.extend(&(0 as i16).to_le_bytes());
        buf.extend(&(0 as i16).to_le_bytes());
        buf
    }
    fn build_mock_grammar(bytes: &[u8]) -> Grammar {
        let mut grammar = Grammar::new(bytes, 0).expect("Failed to create grammar");
        let character_category =
            CharacterCategory::from_file(&PathBuf::from(CHAR_CATEGORY_FILE_PATH))
                .expect("Failed to load character category");
        grammar.set_character_category(character_category);
        grammar
    }
    fn set_up_builder<'a>(grammar: &'a Grammar) -> Utf8InputTextBuilder<'a, 'static> {
        Utf8InputTextBuilder::new(TEXT, grammar)
    }

    #[test]
    fn get_original_text() {
        let bytes = build_mock_bytes();
        let grammar = build_mock_grammar(&bytes);
        let builder = set_up_builder(&grammar);
        assert_eq!(TEXT, builder.original);
        assert_eq!(TEXT, builder.modified);

        let input = builder.build();
        assert_eq!(TEXT, input.original);
        assert_eq!(TEXT, input.modified);
    }

    #[test]
    fn get_original_index() {
        let bytes = build_mock_bytes();
        let grammar = build_mock_grammar(&bytes);
        let builder = set_up_builder(&grammar);
        let input = builder.build();
        assert_eq!(32, input.modified.len());

        for (i, &v) in [
            0, 2, 2, 5, 5, 5, 6, 7, 10, 10, 10, 11, 12, 13, 16, 16, 16, 19, 19, 19, 23, 23, 23, 23,
            26, 26, 26, 29, 29, 29, 32, 32, 32,
        ]
        .iter()
        .enumerate()
        {
            assert_eq!(v, input.get_original_index(i));
        }
    }

    #[test]
    fn get_char_category_types() {
        let bytes = build_mock_bytes();
        let grammar = build_mock_grammar(&bytes);
        let builder = set_up_builder(&grammar);
        let input = builder.build();
        assert!(input
            .get_char_category_types(0)
            .contains(CategoryType::ALPHA));
        assert!(input
            .get_char_category_types(2)
            .contains(CategoryType::ALPHA));
        assert!(input
            .get_char_category_types(5)
            .contains(CategoryType::ALPHA));
        assert!(input
            .get_char_category_types(6)
            .contains(CategoryType::NUMERIC));
        assert!(input
            .get_char_category_types(7)
            .contains(CategoryType::HIRAGANA));
        assert!(input
            .get_char_category_types(9)
            .contains(CategoryType::HIRAGANA));
        assert!(input
            .get_char_category_types(10)
            .contains(CategoryType::NUMERIC));
        assert!(input
            .get_char_category_types(13)
            .contains(CategoryType::KANJI));
        assert!(input
            .get_char_category_types(18)
            .contains(CategoryType::KANJI));
        assert!(input
            .get_char_category_types(19)
            .contains(CategoryType::DEFAULT));
        assert!(input
            .get_char_category_types(22)
            .contains(CategoryType::DEFAULT));
        assert!(input
            .get_char_category_types(23)
            .contains(CategoryType::KATAKANA));
        assert!(input
            .get_char_category_types(26)
            .contains(CategoryType::KATAKANA));
        assert!(input
            .get_char_category_types(31)
            .contains(CategoryType::KATAKANA));
    }

    #[test]
    fn get_char_category_continuous_length() {
        let bytes = build_mock_bytes();
        let grammar = build_mock_grammar(&bytes);
        let builder = set_up_builder(&grammar);
        let input = builder.build();
        assert_eq!(6, input.get_char_category_continuous_length(0));
        assert_eq!(5, input.get_char_category_continuous_length(1));
        assert_eq!(4, input.get_char_category_continuous_length(2));
        assert_eq!(1, input.get_char_category_continuous_length(5));
        assert_eq!(1, input.get_char_category_continuous_length(6));
        assert_eq!(3, input.get_char_category_continuous_length(7));
        assert_eq!(3, input.get_char_category_continuous_length(10));
        assert_eq!(2, input.get_char_category_continuous_length(11));
        assert_eq!(1, input.get_char_category_continuous_length(12));
        assert_eq!(4, input.get_char_category_continuous_length(19));
        assert_eq!(1, input.get_char_category_continuous_length(22));
        assert_eq!(9, input.get_char_category_continuous_length(23));
        assert_eq!(6, input.get_char_category_continuous_length(26));
        assert_eq!(1, input.get_char_category_continuous_length(31));
    }

    #[test]
    fn replace_with_same_length() {
        let bytes = build_mock_bytes();
        let grammar = build_mock_grammar(&bytes);
        let mut builder = set_up_builder(&grammar);
        builder.replace(8..10, "ああ");
        assert_eq!(TEXT, builder.original);
        assert_eq!("âｂC1あ234ああ𡈽アｺﾞ", builder.modified);
        let input = builder.build();
        assert_eq!(TEXT, input.original);
        assert_eq!("âｂC1あ234ああ𡈽アｺﾞ", input.modified);
        assert_eq!(32, input.modified.len());

        assert_eq!(0, input.get_original_index(0));
        assert_eq!(12, input.get_original_index(12));
        assert_eq!(13, input.get_original_index(13));
        assert_eq!(19, input.get_original_index(14));
        assert_eq!(19, input.get_original_index(15));
        assert_eq!(19, input.get_original_index(16));
        assert_eq!(19, input.get_original_index(17));
        assert_eq!(19, input.get_original_index(18));
        assert_eq!(19, input.get_original_index(19));
        assert_eq!(32, input.get_original_index(32));
    }

    #[test]
    fn replace_with_deletion() {
        let bytes = build_mock_bytes();
        let grammar = build_mock_grammar(&bytes);
        let mut builder = set_up_builder(&grammar);
        builder.replace(8..10, "あ");
        assert_eq!(TEXT, builder.original);
        assert_eq!("âｂC1あ234あ𡈽アｺﾞ", builder.modified);
        let input = builder.build();
        assert_eq!(TEXT, input.original);
        assert_eq!("âｂC1あ234あ𡈽アｺﾞ", input.modified);
        assert_eq!(29, input.modified.len());

        assert_eq!(0, input.get_original_index(0));
        assert_eq!(12, input.get_original_index(12));
        assert_eq!(13, input.get_original_index(13));
        assert_eq!(19, input.get_original_index(14));
        assert_eq!(19, input.get_original_index(15));
        assert_eq!(19, input.get_original_index(16));
        assert_eq!(23, input.get_original_index(17));
        assert_eq!(23, input.get_original_index(18));
        assert_eq!(32, input.get_original_index(29));
    }

    #[test]
    fn replace_with_insertion() {
        let bytes = build_mock_bytes();
        let grammar = build_mock_grammar(&bytes);
        let mut builder = set_up_builder(&grammar);
        builder.replace(8..10, "あああ");
        assert_eq!(TEXT, builder.original);
        assert_eq!("âｂC1あ234あああ𡈽アｺﾞ", builder.modified);
        let input = builder.build();
        assert_eq!(TEXT, input.original);
        assert_eq!("âｂC1あ234あああ𡈽アｺﾞ", input.modified);
        assert_eq!(35, input.modified.len());

        assert_eq!(0, input.get_original_index(0));
        assert_eq!(12, input.get_original_index(12));
        assert_eq!(13, input.get_original_index(13)); // >あ< ああ
        assert_eq!(19, input.get_original_index(14));
        assert_eq!(19, input.get_original_index(15));
        assert_eq!(19, input.get_original_index(16)); // ああ >あ<
        assert_eq!(19, input.get_original_index(17));
        assert_eq!(19, input.get_original_index(18));
        assert_eq!(19, input.get_original_index(19)); // ああ >あ<
        assert_eq!(19, input.get_original_index(20));
        assert_eq!(19, input.get_original_index(21));
        assert_eq!(19, input.get_original_index(22)); // あああ >
        assert_eq!(32, input.get_original_index(35));
    }

    #[test]
    fn replace_multiple_times() {
        let bytes = build_mock_bytes();
        let grammar = build_mock_grammar(&bytes);
        let mut builder = set_up_builder(&grammar);
        builder.replace(0..1, "a");
        builder.replace(1..2, "b");
        builder.replace(2..3, "c");
        builder.replace(10..11, "土");
        builder.replace(12..14, "ゴ");
        let input = builder.build();
        assert_eq!(TEXT, input.original);
        assert_eq!("abc1あ234漢字土アゴ", input.modified);
        assert_eq!(25, input.modified.len());

        for (i, &v) in [
            0, 2, 5, 6, 7, 10, 10, 10, 11, 12, 13, 16, 16, 16, 19, 19, 19, 23, 23, 23, 26, 26, 26,
            32, 32, 32,
        ]
        .iter()
        .enumerate()
        {
            assert_eq!(v, input.get_original_index(i));
        }
    }

    #[test]
    fn get_byte_length_by_code_points() {
        let bytes = build_mock_bytes();
        let grammar = build_mock_grammar(&bytes);
        let builder = set_up_builder(&grammar);
        let input = builder.build();
        assert_eq!(2, input.get_code_points_offset_length(0, 1));
        assert_eq!(7, input.get_code_points_offset_length(0, 4));
        assert_eq!(1, input.get_code_points_offset_length(10, 1));
        assert_eq!(1, input.get_code_points_offset_length(11, 1));
        assert_eq!(1, input.get_code_points_offset_length(12, 1));
        assert_eq!(6, input.get_code_points_offset_length(13, 2));
        assert_eq!(4, input.get_code_points_offset_length(19, 1));
        assert_eq!(9, input.get_code_points_offset_length(23, 3));
    }

    #[test]
    fn code_point_count() {
        let bytes = build_mock_bytes();
        let grammar = build_mock_grammar(&bytes);
        let builder = set_up_builder(&grammar);
        let input = builder.build();
        assert_eq!(1, input.code_point_count(0..2));
        assert_eq!(4, input.code_point_count(0..7));
        assert_eq!(2, input.code_point_count(13..19));
    }

    #[test]
    fn can_bow() {
        let bytes = build_mock_bytes();
        let grammar = build_mock_grammar(&bytes);
        let builder = set_up_builder(&grammar);
        let input = builder.build();
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
        let bytes = build_mock_bytes();
        let grammar = build_mock_grammar(&bytes);
        let builder = set_up_builder(&grammar);
        let input = builder.build();
        assert_eq!(6, input.get_word_candidate_length(0));
        assert_eq!(1, input.get_word_candidate_length(6));
        assert_eq!(4, input.get_word_candidate_length(19));
        assert_eq!(3, input.get_word_candidate_length(29));
    }
}
