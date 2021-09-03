use super::*;
use serde_json::Value;
use std::path::PathBuf;

use sudachi::input_text::utf8_input_text_builder::Utf8InputTextBuilder;

const TEST_RESOURCE_DIR_PATH: &str = "tests/resources/";

#[test]
fn ignore_yomigana_at_middle() {
    let original = "徳島（とくしま）に行く";
    let normalized = "徳島に行く";

    let settings = build_mock_setting();
    let config = Config::default();
    let bytes = build_mock_bytes();
    let grammar = build_mock_grammar(&bytes);
    let mut plugin = IgnoreYomiganaPlugin::default();
    plugin
        .set_up(&settings, &config, &grammar)
        .expect("Failed to setup plugin");
    let mut builder = Utf8InputTextBuilder::new(original, &grammar);
    plugin.rewrite(&mut builder);

    let text = builder.build();
    assert_eq!(original, text.original);
    assert_eq!(normalized, text.modified);
    assert_eq!(0, text.get_original_index(0));
    assert_eq!(3, text.get_original_index(3));
    assert_eq!(24, text.get_original_index(6));
    assert_eq!(27, text.get_original_index(9));
    assert_eq!(30, text.get_original_index(12));
}

#[test]
fn ignore_yomigana_at_end() {
    let original = "徳島（とくしま）";
    let normalized = "徳島";

    let settings = build_mock_setting();
    let config = Config::default();
    let bytes = build_mock_bytes();
    let grammar = build_mock_grammar(&bytes);
    let mut plugin = IgnoreYomiganaPlugin::default();
    plugin
        .set_up(&settings, &config, &grammar)
        .expect("Failed to setup plugin");
    let mut builder = Utf8InputTextBuilder::new(original, &grammar);
    plugin.rewrite(&mut builder);

    let text = builder.build();
    assert_eq!(original, text.original);
    assert_eq!(normalized, text.modified);
    assert_eq!(0, text.get_original_index(0));
    assert_eq!(3, text.get_original_index(3));
}

#[test]
fn ignore_yomigana_multiple() {
    let original = "徳島（とくしま）に行（い）く";
    let normalized = "徳島に行く";

    let settings = build_mock_setting();
    let config = Config::default();
    let bytes = build_mock_bytes();
    let grammar = build_mock_grammar(&bytes);
    let mut plugin = IgnoreYomiganaPlugin::default();
    plugin
        .set_up(&settings, &config, &grammar)
        .expect("Failed to setup plugin");
    let mut builder = Utf8InputTextBuilder::new(original, &grammar);
    plugin.rewrite(&mut builder);

    let text = builder.build();
    assert_eq!(original, text.original);
    assert_eq!(normalized, text.modified);
    assert_eq!(0, text.get_original_index(0));
    assert_eq!(3, text.get_original_index(3));
    assert_eq!(24, text.get_original_index(6));
    assert_eq!(27, text.get_original_index(9));
    assert_eq!(39, text.get_original_index(12));
}

#[test]
fn ignore_yomigana_multiple_brace_types() {
    let original = "徳島(とくしま)に行（い）く";
    let normalized = "徳島に行く";

    let settings = build_mock_setting();
    let config = Config::default();
    let bytes = build_mock_bytes();
    let grammar = build_mock_grammar(&bytes);
    let mut plugin = IgnoreYomiganaPlugin::default();
    plugin
        .set_up(&settings, &config, &grammar)
        .expect("Failed to setup plugin");
    let mut builder = Utf8InputTextBuilder::new(original, &grammar);
    plugin.rewrite(&mut builder);

    let text = builder.build();
    assert_eq!(original, text.original);
    assert_eq!(normalized, text.modified);
    assert_eq!(0, text.get_original_index(0));
    assert_eq!(3, text.get_original_index(3));
    assert_eq!(20, text.get_original_index(6));
    assert_eq!(23, text.get_original_index(9));
    assert_eq!(35, text.get_original_index(12));
}

#[test]
fn dont_ignore_not_yomigana() {
    let original = "徳島に（よく）行く";
    let normalized = "徳島に（よく）行く";

    let settings = build_mock_setting();
    let config = Config::default();
    let bytes = build_mock_bytes();
    let grammar = build_mock_grammar(&bytes);
    let mut plugin = IgnoreYomiganaPlugin::default();
    plugin
        .set_up(&settings, &config, &grammar)
        .expect("Failed to setup plugin");
    let mut builder = Utf8InputTextBuilder::new(original, &grammar);
    plugin.rewrite(&mut builder);

    let text = builder.build();
    assert_eq!(original, text.original);
    assert_eq!(normalized, text.modified);
    assert_eq!(0, text.get_original_index(0));
    assert_eq!(3, text.get_original_index(3));
    assert_eq!(6, text.get_original_index(6));
    assert_eq!(9, text.get_original_index(9));
    assert_eq!(24, text.get_original_index(24));
}

#[test]
fn dont_ignore_too_long() {
    let original = "徳島（ながいよみ）に行く";
    let normalized = "徳島（ながいよみ）に行く";

    let settings = build_mock_setting();
    let config = Config::default();
    let bytes = build_mock_bytes();
    let grammar = build_mock_grammar(&bytes);
    let mut plugin = IgnoreYomiganaPlugin::default();
    plugin
        .set_up(&settings, &config, &grammar)
        .expect("Failed to setup plugin");
    let mut builder = Utf8InputTextBuilder::new(original, &grammar);
    plugin.rewrite(&mut builder);

    let text = builder.build();
    assert_eq!(original, text.original);
    assert_eq!(normalized, text.modified);
    assert_eq!(0, text.get_original_index(0));
    assert_eq!(3, text.get_original_index(3));
    assert_eq!(6, text.get_original_index(6));
    assert_eq!(9, text.get_original_index(9));
    assert_eq!(33, text.get_original_index(33));
}

fn build_mock_setting() -> Value {
    let data = r#"
        {
            "leftBrackets": ["(", "（"],
            "rightBrackets": [")", "）"],
            "maxYomiganaLength": 4
        }
    "#;
    serde_json::from_str(data).expect("Failed to parse test settings")
}
fn build_character_category() -> CharacterCategory {
    let char_cat_file_path = PathBuf::from(TEST_RESOURCE_DIR_PATH.to_string() + "char.def");
    CharacterCategory::from_file(char_cat_file_path).expect("Failed to load character category")
}
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
    let char_cat = build_character_category();
    grammar.set_character_category(char_cat);
    grammar
}
