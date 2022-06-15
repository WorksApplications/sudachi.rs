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

use crate::dic::character_category::CharacterCategory;
use crate::dic::grammar::Grammar;
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

pub fn build_mock_bytes() -> Vec<u8> {
    let mut buf = Vec::new();
    // encode pos for oov
    buf.extend(&(1 as i16).to_le_bytes());
    let pos = vec!["補助記号", "一般", "*", "*", "*", "*"];
    for s in pos {
        let utf16: Vec<_> = s.encode_utf16().collect();
        buf.extend(&(utf16.len() as u8).to_le_bytes());
        for c in utf16 {
            buf.extend(&(c).to_le_bytes());
        }
    }
    // set 10 for left and right id sizes
    buf.extend(&(10 as i16).to_le_bytes());
    buf.extend(&(10 as i16).to_le_bytes());
    for i in 0..10 {
        for j in 0..10 {
            let val = i * 100 + j;
            buf.extend(&(val as i16).to_le_bytes());
        }
    }

    buf
}

pub fn build_mock_grammar(bytes: &[u8]) -> Grammar {
    let mut grammar = Grammar::parse(bytes, 0).expect("Failed to create grammar");
    grammar.set_character_category(char_cats());
    grammar
}

lazy_static! {
    pub static ref GRAMMAR_BYTES: Vec<u8> = build_mock_bytes();
    pub static ref GRAMMAR: Grammar<'static> = build_mock_grammar(&GRAMMAR_BYTES);
}

pub fn input_text(data: impl AsRef<str>) -> InputBuffer {
    let mut buf = InputBuffer::from(data.as_ref());
    buf.build(&GRAMMAR).expect("does not fail");
    buf
}
