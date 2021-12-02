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

use crate::common::TestStatefulTokenizer;
use std::ops::Deref;
use sudachi::prelude::Mode;

mod common;

const AB_DIC: &[u8] = include_bytes!("resources/split_alpha.csv");

#[test]
fn split_at_analysis_time() {
    let mut tok = TestStatefulTokenizer::builder(AB_DIC).mode(Mode::A).build();
    let res = tok.tokenize("ＡＢ");
    assert_eq!(res.len(), 2);
    assert_eq!(res.get(0).surface().deref(), "Ａ");
    assert_eq!(res.get(1).surface().deref(), "Ｂ");
}

#[test]
fn split_after_analysis() {
    let mut tok = TestStatefulTokenizer::builder(AB_DIC).build();
    let res = tok.tokenize("ＡＢ");
    assert_eq!(res.len(), 1);
    let mut res2 = res.empty_clone();
    assert!(res.get(0).split_into(Mode::A, &mut res2).unwrap());
    assert_eq!(res2.get(0).surface().deref(), "Ａ");
    assert_eq!(res2.get(1).surface().deref(), "Ｂ");
}
