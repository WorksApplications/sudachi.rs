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

use claim::assert_matches;
use serde_json::{Map, Value};

use crate::config::Config;
use crate::input_text::InputTextIndex;
use crate::test::zero_grammar;

use super::*;

const TEST_RESOURCE_DIR: &str = "tests/resources/";
const ORIGINAL_TEXT: &str = "ÂＢΓД㈱ｶﾞウ゛⼼Ⅲ";
const NORMALIZED_TEXT: &str = "âbγд(株)ガヴ⼼ⅲ";

#[test]
fn after_rewrite() {
    let plugin = test_plugin();
    let mut text = InputBuffer::from(ORIGINAL_TEXT);
    plugin.rewrite(&mut text).expect("succeeds");

    assert_eq!(NORMALIZED_TEXT, text.current());
    assert_eq!(24, text.current().len());
    let expected = b"\xc3\xa2\x62\xce\xb3\xd0\xb4\x28\xe6\xa0\xaa\x29\xe3\x82\xac\xe3\x83\xb4\xe2\xbc\xbc\xe2\x85\xb2";
    assert_eq!(expected, text.current().as_bytes());
    assert_eq!("Â", text.orig_slice(0..2));
    assert_eq!("Ｂ", text.orig_slice(2..3));
    assert_eq!("Γ", text.orig_slice(3..5));
    assert_eq!("Д", text.orig_slice(5..7));
    assert_eq!("㈱", text.orig_slice(7..12));
    assert_eq!("ｶﾞ", text.orig_slice(12..15));
    assert_eq!("ウ゛", text.orig_slice(15..18));
    assert_eq!("⼼", text.orig_slice(18..21));
    assert_eq!("Ⅲ", text.orig_slice(21..24));
}

#[test]
fn ignore_list_two_chars() {
    let data = "12";
    let mut plugin = DefaultInputTextPlugin::default();
    let result = plugin.read_rewrite_lists(data.as_bytes());
    assert_matches!(result, Err(SudachiError::InvalidDataFormat(0, _)))
}

#[test]
fn replace_list_three_entries() {
    let data = "12 21 31";
    let mut plugin = DefaultInputTextPlugin::default();
    let result = plugin.read_rewrite_lists(data.as_bytes());
    assert_matches!(result, Err(SudachiError::InvalidDataFormat(0, _)))
}

#[test]
fn replace_list_duplicates() {
    let data = "
    12 31
    12 31";

    let mut plugin = DefaultInputTextPlugin::default();
    let result = plugin.read_rewrite_lists(data.as_bytes());
    assert_matches!(result, Err(SudachiError::InvalidDataFormat(2, _)));
}

#[test]
fn rewrite_hiragana() {
    let plugin = test_plugin();
    let mut buffer = InputBuffer::from("ひらがな");
    plugin.rewrite(&mut buffer).expect("rewrite failed");
    assert_eq!(buffer.current(), "ひらがな");
}

#[test]
fn nfkc_works() {
    let plugin = test_plugin();
    let mut buffer = InputBuffer::from("ひＢら①がⅢな");
    plugin.rewrite(&mut buffer).expect("rewrite failed");
    assert_eq!(buffer.current(), "ひbら1がⅲな");
}

#[test]
fn lowercasing_works_simple() {
    let plugin = test_plugin();
    let mut buffer = InputBuffer::from("ひЗДらTESTがЕСЬな");
    plugin.rewrite(&mut buffer).expect("rewrite failed");
    assert_eq!(buffer.current(), "ひздらtestがесьな");
}

#[test]
fn lowercasing_works_difficult() {
    let plugin = test_plugin();
    let mut buffer = InputBuffer::from("ひらİがẞなΣ");
    plugin.rewrite(&mut buffer).expect("rewrite failed");
    assert_eq!(buffer.current(), "ひらi\u{307}がßなσ");
}

#[test]
fn replacement_works() {
    let plugin = test_plugin();
    let mut buffer = InputBuffer::from("ウ゛");
    plugin.rewrite(&mut buffer).expect("rewrite failed");
    assert_eq!(buffer.current(), "ヴ");
}

#[test]
fn full_normalization_works() {
    let plugin = test_plugin();
    let mut buffer = InputBuffer::from(ORIGINAL_TEXT);
    plugin.rewrite(&mut buffer).expect("rewrite failed");
    assert_eq!(buffer.current(), NORMALIZED_TEXT);
}

fn test_plugin() -> DefaultInputTextPlugin {
    let settings = build_mock_setting_from_file_name("rewrite.def");
    let config = Config::default();
    let grammar = zero_grammar();
    let mut plugin = DefaultInputTextPlugin::default();
    plugin
        .set_up(&settings, &config, &grammar)
        .expect("Failed to setup plugin");
    plugin
}

fn build_mock_setting_from_file_name(rewrite_def_file: &str) -> Value {
    let mut map = Map::default();
    map.insert(
        "rewriteDef".to_string(),
        Value::String(TEST_RESOURCE_DIR.to_string() + rewrite_def_file),
    );
    Value::Object(map)
}
