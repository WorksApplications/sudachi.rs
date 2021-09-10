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
use serde_json::{Map, Value};
use std::io::{Seek, SeekFrom, Write};
use tempfile::tempfile;

use sudachi::config::Config;
use sudachi::dic::grammar::Grammar;
use sudachi::input_text::utf8_input_text_builder::Utf8InputTextBuilder;

const TEST_RESOURCE_DIR: &str = "tests/resources/";
const ORIGINAL_TEXT: &str = "ÂＢΓД㈱ｶﾞウ゛⼼Ⅲ";
const NORMALIZED_TEXT: &str = "âbγд(株)ガヴ⼼ⅲ";

#[test]
fn before_rewrite() {
    let bytes = build_mock_bytes();
    let grammar = build_mock_grammar(&bytes);
    let builder = Utf8InputTextBuilder::new(ORIGINAL_TEXT, &grammar);

    let text = builder.build();
    assert_eq!(30, text.modified.as_bytes().len());
    let expected = b"\xc3\x82\xef\xbc\xa2\xce\x93\xd0\x94\xe3\x88\xb1\xef\xbd\xb6\xef\xbe\x9e\xe3\x82\xa6\xe3\x82\x9b\xe2\xbc\xbc\xe2\x85\xa2";
    assert_eq!(expected, text.modified.as_bytes());

    assert_eq!(0, text.get_original_index(0));
    assert_eq!(2, text.get_original_index(1));
    assert_eq!(2, text.get_original_index(2));
    assert_eq!(5, text.get_original_index(4));
    assert_eq!(7, text.get_original_index(7));
    assert_eq!(12, text.get_original_index(12));
    assert_eq!(24, text.get_original_index(24));
    assert_eq!(27, text.get_original_index(27));
    assert_eq!(30, text.get_original_index(30));
}

#[test]
fn after_rewrite() {
    let settings = build_mock_setting_from_file_name("rewrite.def");
    let config = Config::default();
    let bytes = build_mock_bytes();
    let grammar = build_mock_grammar(&bytes);
    let mut plugin = DefaultInputTextPlugin::default();
    plugin
        .set_up(&settings, &config, &grammar)
        .expect("Failed to setup plugin");
    let mut builder = Utf8InputTextBuilder::new(ORIGINAL_TEXT, &grammar);
    plugin.rewrite(&mut builder);

    let text = builder.build();
    assert_eq!(NORMALIZED_TEXT, text.modified);
    assert_eq!(24, text.modified.as_bytes().len());
    let expected = b"\xc3\xa2\x62\xce\xb3\xd0\xb4\x28\xe6\xa0\xaa\x29\xe3\x82\xac\xe3\x83\xb4\xe2\xbc\xbc\xe2\x85\xb2";
    assert_eq!(expected, text.modified.as_bytes());
    assert_eq!(0, text.get_original_index(0));
    assert_eq!(2, text.get_original_index(1));
    assert_eq!(2, text.get_original_index(2));
    assert_eq!(5, text.get_original_index(3));
    assert_eq!(9, text.get_original_index(7));
    assert_eq!(12, text.get_original_index(8));
    assert_eq!(12, text.get_original_index(11));
    assert_eq!(18, text.get_original_index(15));
    assert_eq!(24, text.get_original_index(17));
}

#[test]
#[should_panic]
fn invalid_format_ignorelist() {
    let mut file = tempfile().expect("Failed to get temporary file");
    writeln!(file, "# there are two characters in ignore list").unwrap();
    writeln!(file, "12").unwrap();
    file.flush().expect("Failed to flush");
    file.seek(SeekFrom::Start(0)).expect("Failed to seek");

    let mut plugin = DefaultInputTextPlugin::default();
    plugin
        .read_rewrite_lists(BufReader::new(file))
        .expect("Failed to read rewrite lists");
}

#[test]
#[should_panic]
fn invalid_format_replacelist() {
    let mut file = tempfile().expect("Failed to get temporary file");
    writeln!(file, "# there are three columns in replace list").unwrap();
    writeln!(file, "12 21 31").unwrap();
    file.flush().expect("Failed to flush");
    file.seek(SeekFrom::Start(0)).expect("Failed to seek");

    let mut plugin = DefaultInputTextPlugin::default();
    plugin
        .read_rewrite_lists(BufReader::new(file))
        .expect("Failed to read rewrite lists");
}

#[test]
#[should_panic]
fn duplicated_lines_replacelist() {
    let mut file = tempfile().expect("Failed to get temporary file");
    writeln!(file, "# there are a duplicated replacement.").unwrap();
    writeln!(file, "12 21").unwrap();
    writeln!(file, "12 31").unwrap();
    file.flush().expect("Failed to flush");
    file.seek(SeekFrom::Start(0)).expect("Failed to seek");

    let mut plugin = DefaultInputTextPlugin::default();
    plugin
        .read_rewrite_lists(BufReader::new(file))
        .expect("Failed to read rewrite lists");
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
    Grammar::new(bytes, 0).expect("Failed to create grammar")
}
fn build_mock_setting_from_file_name(rewrite_def_file: &str) -> Value {
    let mut map = Map::default();
    map.insert(
        "rewriteDef".to_string(),
        Value::String(TEST_RESOURCE_DIR.to_string() + rewrite_def_file),
    );
    Value::Object(map)
}
