/*
 *  Copyright (c) 2021-2026 Works Applications Co., Ltd.
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

use crate::dic::character_category::CharacterCategory;
use crate::dic::connect::ConnectionMatrix;
use crate::dic::grammar::Grammar;
use crate::dic::pos::PosList;
use crate::input_text::InputBuffer;
use lazy_static::lazy_static;

pub const ALL_KANJI_CAT: &str = "
0x0061..0x007A ALPHA    #a-z
0x3041..0x309F  KANJI # HIRAGANA
0x30A1..0x30FF  KANJINUMERIC # KATAKANA
";

pub fn char_cats() -> CharacterCategory {
    CharacterCategory::from_reader(ALL_KANJI_CAT.as_bytes()).unwrap()
}

pub fn build_mock_connection_bytes() -> Vec<u8> {
    let mut buf = Vec::new();
    // set 10 for left and right id sizes
    buf.extend(&10_i16.to_le_bytes());
    buf.extend(&10_i16.to_le_bytes());
    for i in 0..10 {
        for j in 0..10 {
            let val = i * 100 + j;
            buf.extend(&(val as i16).to_le_bytes());
        }
    }

    buf
}

pub fn build_mock_grammar(connection_bytes: &[u8]) -> Grammar<'_> {
    let mut pos_list = PosList::default();
    pos_list.push(vec![
        "補助記号".to_string(),
        "一般".to_string(),
        "*".to_string(),
        "*".to_string(),
        "*".to_string(),
        "*".to_string(),
    ]);

    let connection =
        ConnectionMatrix::from_bytes(connection_bytes).expect("Failed to parse connection matrix");

    let mut grammar = Grammar::from_parts(pos_list, connection);
    grammar.set_character_category(char_cats());
    grammar
}

lazy_static! {
    pub static ref CONNECTION_BYTES: Vec<u8> = build_mock_connection_bytes();
    pub static ref GRAMMAR: Grammar<'static> = build_mock_grammar(&CONNECTION_BYTES);
}

pub fn input_text(data: impl AsRef<str>) -> InputBuffer {
    let mut buf = InputBuffer::from(data.as_ref());
    buf.build(&GRAMMAR).expect("does not fail");
    buf
}
