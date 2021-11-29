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

extern crate lazy_static;

mod common;
use common::{TestTokenizer, GRAMMAR};

#[test]
fn get_part_of_speech_size() {
    // pos from system test dict
    assert_eq!(8, GRAMMAR.pos_list.len());

    // user test dict contains another pos
    let tokenizer = TestTokenizer::new();
    assert_eq!(9, tokenizer.dict().grammar().pos_list.len());
}

#[test]
fn get_part_of_speech_string() {
    let pos = &GRAMMAR.pos_list[0];
    assert!(!pos.is_empty());
    assert_eq!("助動詞", pos[0]);
}

// fn creat_with_merging_settings
// fn creat_with_merging_null_settings
