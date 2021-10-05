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

use crate::dic::grammar::Grammar;

const ZERO_GRAMMAR_BYTES: &[u8] = &[0u8; 6];

pub fn zero_grammar() -> Grammar<'static> {
    Grammar::new(ZERO_GRAMMAR_BYTES, 0).expect("Failed to make grammar")
}

#[test]
fn make_zero_grammar() {
    let _ = zero_grammar();
}
