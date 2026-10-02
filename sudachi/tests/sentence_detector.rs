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
use common::TestTokenizer;

use sudachi::dic::binary_loader::LoadedDictionary;
use sudachi::dic::build::DictBuilder;
use sudachi::sentence_detector::{NonBreakChecker, SentenceDetector};
use sudachi::sentence_splitter::{SentenceSplitter, SplitSentences};

#[test]
fn get_eos_with_non_break_checker() {
    let text = "ばな。なです。";
    let tokenizer = TestTokenizer::new();
    let lexicon = tokenizer.dict().lexicon();
    let checker = NonBreakChecker::new(lexicon);

    let sd = SentenceDetector::new();
    assert_eq!(sd.get_eos(text, Some(&checker)).unwrap(), 21);
}

const PERIOD_LEXICON: &str = "\
index_form,left_id,right_id,cost,headword,pos1,pos2,pos3,pos4,pos5,pos6,reading_form,normalized_form,dictionary_form,split_a,split_b,split_c,word_structure,synonym_groups
。,8,8,2914,,補助記号,句点,*,*,*,*,。,,,,,,,
娘。,8,8,2914,,名詞,普通名詞,一般,*,*,*,ムスメ,,,,,,,
な。な,8,8,2914,,名詞,普通名詞,一般,*,*,*,ナナ,,,,,,,
。な,8,8,2914,,名詞,普通名詞,一般,*,*,*,ナ,,,,,,,
";

#[test]
fn get_eos_with_non_break_checker_single_char_word_on_boundary() {
    let mut builder = DictBuilder::new_system();
    builder
        .read_conn(include_bytes!("resources/matrix_10x10.def"))
        .unwrap();
    builder.read_lexicon(PERIOD_LEXICON.as_bytes()).unwrap();
    builder.resolve().unwrap();
    let mut bytes = Vec::new();
    builder.compile(&mut bytes).unwrap();
    let dic = LoadedDictionary::load_system(&bytes).unwrap();
    let lexicon = &dic.lexicon_set;

    let checker = NonBreakChecker::new(lexicon);
    let sd = SentenceDetector::new();

    // "。" is a one-character word ending exactly on the boundary: split there
    assert_eq!(
        sd.get_eos("今日は晴れ。明日は雨。", Some(&checker))
            .unwrap(),
        18
    );
    // "娘。" is a longer word ending on the boundary: do not split (モーニング娘。)
    assert_eq!(
        sd.get_eos("モーニング娘。です。", Some(&checker)).unwrap(),
        30
    );
    // "な。な" crosses the boundary: do not split
    assert_eq!(sd.get_eos("ばな。なです。", Some(&checker)).unwrap(), 21);
    // "。な" starts at the same position as "。" and crosses the boundary: do not split
    assert_eq!(sd.get_eos("あ。なに。", Some(&checker)).unwrap(), 15);

    let splitter = SentenceSplitter::new().with_checker(lexicon);
    let sentences: Vec<&str> = splitter
        .split("今日は晴れ。明日は雨。")
        .map(|(_, s)| s)
        .collect();
    assert_eq!(sentences, ["今日は晴れ。", "明日は雨。"]);
}
