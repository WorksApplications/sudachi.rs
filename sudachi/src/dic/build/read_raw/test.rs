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
use crate::dic::build::error::DicWriteError;
use crate::error::SudachiError;
use claim::assert_matches;
use std::fmt::Write;

#[test]
fn decode_plain() {
    assert_eq!(unescape("").unwrap(), "");
    assert_eq!(unescape("a").unwrap(), "a");
    assert_eq!(unescape("あ").unwrap(), "あ");
}

#[test]
fn decode_escape_1() {
    assert_eq!(unescape("\\u0020").unwrap(), "\u{20}");
    assert_eq!(unescape("\\u{20}").unwrap(), "\u{20}");
    assert_eq!(unescape("\\u{1f49e}").unwrap(), "💞");
}

#[test]
fn decode_escape_2() {
    assert_eq!(unescape("\\u020f").unwrap(), "\u{20f}");
    assert_eq!(unescape("\\u{20}f").unwrap(), "\u{20}f");
}

#[test]
fn decode_escape_3() {
    assert_eq!(unescape("f\\u0020").unwrap(), "f\u{20}");
    assert_eq!(unescape("f\\u{20}").unwrap(), "f\u{20}");
}

#[test]
fn decode_escape_4() {
    assert_eq!(unescape("\\u100056").unwrap(), "\u{1000}56");
}

#[test]
fn decode_escape_ported() {
    assert_eq!(unescape("a\\u002cc").unwrap(), "a,c");
    assert_eq!(unescape("a\\u{2c}c").unwrap(), "a,c");
}

#[test]
fn decode_escape_fail() {
    assert_eq!(unescape("\\u{10FFFF}").unwrap(), "\u{10FFFF}"); // max character
    claim::assert_matches!(unescape("\\u{110000}"), Err(_));
    claim::assert_matches!(unescape("\\u{FFFFFF}"), Err(_));
}

#[test]
fn parse_split_empty() {
    let mut rdr = LexiconReader::new();
    assert_eq!(rdr.parse_splits("").unwrap().len(), 0);
    assert_eq!(rdr.parse_splits("*").unwrap().len(), 0);
}

#[test]
fn parse_split_sys_ids() {
    let mut rdr = LexiconReader::new();
    let splits = rdr.parse_splits("0/1/2").unwrap();
    assert_eq!(splits.len(), 3);
    assert_eq!(splits[0], SplitUnit::Ref(WordId::new(0, 0)));
    assert_eq!(splits[1], SplitUnit::Ref(WordId::new(0, 1)));
    assert_eq!(splits[2], SplitUnit::Ref(WordId::new(0, 2)));
}

#[test]
fn parse_split_user_ids() {
    let mut rdr = LexiconReader::new();
    let splits = rdr.parse_splits("0/U1/2").unwrap();
    assert_eq!(splits.len(), 3);
    assert_eq!(splits[0], SplitUnit::Ref(WordId::new(0, 0)));
    assert_eq!(splits[1], SplitUnit::Ref(WordId::new(1, 1)));
    assert_eq!(splits[2], SplitUnit::Ref(WordId::new(0, 2)));
}

#[test]
fn parse_kyoto() {
    let mut rdr = LexiconReader::new();
    let data = "京都,6,6,5293,京都,名詞,固有名詞,地名,一般,*,*,キョウト,京都,*,A,*,*,*,*";
    rdr.read_bytes(data.as_bytes()).unwrap();
    let entries = rdr.entries();
    assert_eq!(entries.len(), 1);
    let kyoto = &entries[0];
    assert_eq!("京都", kyoto.surface);
    assert_eq!(0, kyoto.pos);
    assert_eq!(
        "名詞,固有名詞,地名,一般,*,*",
        format!("{:?}", rdr.pos_obj(kyoto.pos).unwrap())
    );
    assert_eq!(6, kyoto.left_id);
    assert_eq!(6, kyoto.right_id);
    assert_eq!(5293, kyoto.cost);
    assert_eq!("キョウト", kyoto.reading());
    assert_eq!(Some("キョウト"), kyoto.reading.as_deref());
    assert_eq!("京都", kyoto.norm_form());
    assert_eq!(None, kyoto.norm_form);
    assert_eq!(Mode::A, kyoto.splitting);
    assert_eq!(0, kyoto.splits_a.len());
    assert_eq!(0, kyoto.splits_b.len());
    assert!(kyoto.should_index());
}

#[test]
fn parse_kyoto_ignored() {
    let mut rdr = LexiconReader::new();
    let data = "京都,-1,-1,5293,京都,名詞,固有名詞,地名,一般,*,*,キョウト,京都,*,A,*,*,*,*";
    rdr.read_bytes(data.as_bytes()).unwrap();
    let entries = rdr.entries();
    assert_eq!(entries.len(), 1);
    let kyoto = &entries[0];
    assert_eq!(false, kyoto.should_index());
}

#[test]
fn parse_kyoto_synonym_opt() {
    let mut rdr = LexiconReader::new();
    // last field is omitted
    let data = "京都,1,1,5293,京都,名詞,固有名詞,地名,一般,*,*,キョウト,京都,*,A,*,*,*";
    rdr.read_bytes(data.as_bytes()).unwrap();
    let entries = rdr.entries();
    assert_eq!(entries.len(), 1);
    let kyoto = &entries[0];
    assert_eq!(0, kyoto.synonyms.len());
}

#[test]
fn parse_kyoto_not_enough_fields() {
    let mut rdr = LexiconReader::new();
    // last field is omitted
    let data = "京都,1,1,5293,京都,名詞,固有名詞,地名,一般,*,*,キョウト,京都,*,A,*,*";

    assert_matches!(
        rdr.read_bytes(data.as_bytes()),
        Err(SudachiError::DictionaryCompilationError(DicWriteError {
            cause: DicWriteReason::NoRawField(_),
            line: 1,
            ..
        }))
    );
}

#[test]
fn parse_kyoto_ignored_empty_surface() {
    let mut rdr = LexiconReader::new();
    let data = ",-1,-1,5293,京都,名詞,固有名詞,地名,一般,*,*,キョウト,京都,*,A,*,*,*,*";
    assert_matches!(
        rdr.read_bytes(data.as_bytes()),
        Err(SudachiError::DictionaryCompilationError(DicWriteError {
            cause: DicWriteReason::EmptySurface,
            line: 1,
            ..
        }))
    );
}

#[test]
fn parse_pos_exhausted() {
    let mut rdr = LexiconReader::new();
    let mut data = String::new();
    for i in 0..=MAX_POS_IDS + 1 {
        write!(
            data,
            "x,-1,-1,5293,京都,名詞,固有名詞,地名,一般,*,{},キョウト,京都,*,A,*,*,*,*\n",
            i
        )
        .unwrap()
    }

    assert_matches!(
        rdr.read_bytes(data.as_bytes()),
        Err(SudachiError::DictionaryCompilationError(DicWriteError {
            cause: DicWriteReason::PosLimitExceeded(_),
            ..
        }))
    );
}
