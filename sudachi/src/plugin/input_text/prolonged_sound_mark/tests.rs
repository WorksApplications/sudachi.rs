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
use serde_json::{Map, Value};

use crate::config::Config;
use crate::dic::grammar::Grammar;
use crate::test::zero_grammar;

#[test]
fn combine_continuous_prolonged_sound_mark() {
    let original = "ゴーール";
    let normalized = "ゴール";

    let (_, plugin) = setup();
    let mut text = InputBuffer::from(original);
    plugin.rewrite(&mut text).expect("succeeds");

    assert_eq!(original, text.original());
    assert_eq!(normalized, text.current());
    assert_eq!(9, text.current().len());
    let expected = b"\xe3\x82\xb4\xe3\x83\xbc\xe3\x83\xab";
    assert_eq!(expected, text.current().as_bytes());
    assert_eq!(0, text.get_original_index(0));
    assert_eq!(3, text.get_original_index(3));
    assert_eq!(9, text.get_original_index(6));
    assert_eq!(12, text.get_original_index(9));
}

#[test]
fn combined_continuous_prolonged_sound_marks_at_end() {
    let original = "スーパーー";
    let normalized = "スーパー";

    let (_, plugin) = setup();
    let mut text = InputBuffer::from(original);
    plugin.rewrite(&mut text).expect("succeeds");

    assert_eq!(original, text.original());
    assert_eq!(normalized, text.current());

    assert_eq!(12, text.current().as_bytes().len());
    let expected = b"\xe3\x82\xb9\xe3\x83\xbc\xe3\x83\x91\xe3\x83\xbc";
    assert_eq!(expected, text.current().as_bytes());

    assert_eq!(0, text.get_original_index(0));
    assert_eq!(3, text.get_original_index(3));
    assert_eq!(6, text.get_original_index(6));
    assert_eq!(9, text.get_original_index(9));
    assert_eq!(15, text.get_original_index(12));
}
#[test]
fn combine_continuous_prolonged_sound_marks_multi_times() {
    let original = "エーービーーーシーーーー";
    let normalized = "エービーシー";

    let (_, plugin) = setup();
    let mut text = InputBuffer::from(original);
    plugin.rewrite(&mut text).expect("succeeds");

    assert_eq!(original, text.original());
    assert_eq!(normalized, text.current());

    assert_eq!(18, text.current().as_bytes().len());
    let expected = b"\xe3\x82\xa8\xe3\x83\xbc\xe3\x83\x93\xe3\x83\xbc\xe3\x82\xb7\xe3\x83\xbc";
    assert_eq!(expected, text.current().as_bytes());

    assert_eq!(0, text.get_original_index(0));
    assert_eq!(3, text.get_original_index(3));
    assert_eq!(9, text.get_original_index(6));
    assert_eq!(12, text.get_original_index(9));
    assert_eq!(21, text.get_original_index(12));
    assert_eq!(24, text.get_original_index(15));
    assert_eq!(36, text.get_original_index(18));
}
#[test]
fn combine_continuous_prolonged_sound_marks_multi_symbol_types() {
    let original = "エーービ〜〜〜シ〰〰〰〰";
    let normalized = "エービーシー";

    let (_, plugin) = setup();
    let mut text = InputBuffer::from(original);
    plugin.rewrite(&mut text).expect("succeeds");

    assert_eq!(original, text.original());
    assert_eq!(normalized, text.current());

    assert_eq!(18, text.current().as_bytes().len());
    let expected = b"\xe3\x82\xa8\xe3\x83\xbc\xe3\x83\x93\xe3\x83\xbc\xe3\x82\xb7\xe3\x83\xbc";
    assert_eq!(expected, text.current().as_bytes());

    assert_eq!(0, text.get_original_index(0));
    assert_eq!(3, text.get_original_index(3));
    assert_eq!(9, text.get_original_index(6));
    assert_eq!(12, text.get_original_index(9));
    assert_eq!(21, text.get_original_index(12));
    assert_eq!(24, text.get_original_index(15));
    assert_eq!(36, text.get_original_index(18));
}

#[test]
fn combine_continuous_prolonged_sound_marks_multi_mixed_symbol_types() {
    let original = "エー〜ビ〜〰ーシ〰ー〰〜";
    let normalized = "エービーシー";

    let (_, plugin) = setup();
    let mut text = InputBuffer::from(original);
    plugin.rewrite(&mut text).expect("succeeds");

    assert_eq!(original, text.original());
    assert_eq!(normalized, text.current());

    assert_eq!(18, text.current().as_bytes().len());
    let expected = b"\xe3\x82\xa8\xe3\x83\xbc\xe3\x83\x93\xe3\x83\xbc\xe3\x82\xb7\xe3\x83\xbc";
    assert_eq!(expected, text.current().as_bytes());

    assert_eq!(0, text.get_original_index(0));
    assert_eq!(3, text.get_original_index(3));
    assert_eq!(9, text.get_original_index(6));
    assert_eq!(12, text.get_original_index(9));
    assert_eq!(21, text.get_original_index(12));
    assert_eq!(24, text.get_original_index(15));
    assert_eq!(36, text.get_original_index(18));
}

fn setup<'a>() -> (Grammar<'a>, ProlongedSoundMarkPlugin) {
    let settings = build_mock_setting();
    let config = Config::default();
    let grammar = zero_grammar();
    let mut plugin = ProlongedSoundMarkPlugin::default();
    plugin
        .set_up(&settings, &config, &grammar)
        .expect("Failed to setup plugin");

    (grammar, plugin)
}

fn build_mock_setting() -> Value {
    let mut map = Map::default();
    map.insert(
        "prolongedSoundMarks".to_string(),
        Value::Array(vec![
            Value::String("ー".to_string()),
            Value::String("〜".to_string()),
            Value::String("〰".to_string()),
        ]),
    );
    Value::Object(map)
}
