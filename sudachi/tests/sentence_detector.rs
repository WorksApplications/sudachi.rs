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

#[macro_use]
extern crate lazy_static;

mod common;
use common::TestTokenizer;
use common::GRAMMAR;

use sudachi::input_text::Utf8InputTextBuilder;
use sudachi::sentence_detector::{NonBreakChecker, SentenceDetector};

#[test]
fn get_eos_with_non_break_checker() {
    let text = "ばな。なです。";
    let builder = Utf8InputTextBuilder::new(&text, &GRAMMAR);
    let input = builder.build();
    let tokenizer = TestTokenizer::new();
    let lexicon = tokenizer.dict().lexicon();
    let checker = NonBreakChecker::new(lexicon, &input);

    let sd = SentenceDetector::new();
    assert_eq!(sd.get_eos(text, Some(&checker)).unwrap(), 21);
}
