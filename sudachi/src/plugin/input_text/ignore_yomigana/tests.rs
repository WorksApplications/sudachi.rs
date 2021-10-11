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

use super::*;
use serde_json::Value;
use std::path::PathBuf;

use crate::test::zero_grammar;

const TEST_RESOURCE_DIR_PATH: &str = "tests/resources/";

#[test]
fn ignore_yomigana_at_middle() {
    let original = "徳島（とくしま）に行く";
    let normalized = "徳島に行く";

    let mut text = InputBuffer::from(original);
    let (plugin, _) = setup();
    plugin.rewrite(&mut text).expect("succeeded");

    assert_eq!(original, text.original());
    assert_eq!(normalized, text.current());
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

    let mut text = InputBuffer::from(original);
    let (plugin, _) = setup();
    plugin.rewrite(&mut text).expect("succeeded");

    assert_eq!(original, text.original());
    assert_eq!(normalized, text.current());
    assert_eq!(0, text.get_original_index(0));
    assert_eq!(3, text.get_original_index(3));
}

#[test]
fn ignore_yomigana_multiple() {
    let original = "徳島（とくしま）に行（い）く";
    let normalized = "徳島に行く";

    let mut text = InputBuffer::from(original);
    let (plugin, _) = setup();
    plugin.rewrite(&mut text).expect("succeeded");

    assert_eq!(original, text.original());
    assert_eq!(normalized, text.current());
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

    let mut text = InputBuffer::from(original);
    let (plugin, _) = setup();
    plugin.rewrite(&mut text).expect("succeeded");

    assert_eq!(original, text.original());
    assert_eq!(normalized, text.current());
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

    let mut text = InputBuffer::from(original);
    let (plugin, _) = setup();
    plugin.rewrite(&mut text).expect("succeeded");

    assert_eq!(original, text.original());
    assert_eq!(normalized, text.current());
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

    let mut text = InputBuffer::from(original);
    let (plugin, _) = setup();
    plugin.rewrite(&mut text).expect("succeeded");

    assert_eq!(original, text.original());
    assert_eq!(normalized, text.current());
    assert_eq!(0, text.get_original_index(0));
    assert_eq!(3, text.get_original_index(3));
    assert_eq!(6, text.get_original_index(6));
    assert_eq!(9, text.get_original_index(9));
    assert_eq!(33, text.get_original_index(33));
}

#[test]
fn ignore_hiragana() {
    let (plugin, _) = setup();
    let mut buffer = InputBuffer::from("徳島(とくしま)に行（い）く");
    plugin.rewrite(&mut buffer).expect("should not happen");
    assert_eq!(buffer.current(), "徳島に行く");
}

fn setup() -> (IgnoreYomiganaPlugin, Grammar<'static>) {
    let settings = build_mock_setting();
    let config = Config::default();
    let mut grammar = zero_grammar();
    grammar.set_character_category(build_character_category());
    let mut plugin = IgnoreYomiganaPlugin::default();
    plugin
        .set_up(&settings, &config, &grammar)
        .expect("Failed to setup plugin");
    (plugin, grammar)
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
    CharacterCategory::from_file(&char_cat_file_path).expect("Failed to load character category")
}
