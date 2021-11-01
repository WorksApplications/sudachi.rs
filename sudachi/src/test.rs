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
use lazy_static::lazy_static;

const ZERO_GRAMMAR_BYTES: &[u8] = &[0u8; 6];

/// Returns Grammar with empty data
pub fn zero_grammar() -> Grammar<'static> {
    Grammar::parse(ZERO_GRAMMAR_BYTES, 0).expect("Failed to make grammar")
}

const TEST_CHAR_DEF: &[u8] = include_bytes!("../tests/resources/char.def");

lazy_static! {
    pub static ref CHAR_CAT: CharacterCategory =
        CharacterCategory::from_reader(TEST_CHAR_DEF).unwrap();
}

/// Returns grammar that has test character categories
pub fn cat_grammar() -> Grammar<'static> {
    let mut grammar = zero_grammar();
    grammar.set_character_category(CHAR_CAT.clone());
    grammar
}

#[test]
fn make_zero_grammar() {
    let _ = zero_grammar();
}
