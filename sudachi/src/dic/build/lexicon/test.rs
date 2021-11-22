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
use crate::dic::build::error::DicBuildError;
use crate::dic::build::DictBuilder;
use crate::dic::lexicon::word_infos::WordInfo;
use crate::dic::read::word_info::WordInfoParser;
use crate::error::SudachiError;
use claim::assert_matches;
use std::fmt::Write;

#[test]
fn parse_split_empty() {
    let mut rdr = LexiconReader::new();
    assert_eq!(rdr.parse_splits("").unwrap().0.len(), 0);
    assert_eq!(rdr.parse_splits("*").unwrap().0.len(), 0);
}

#[test]
fn parse_split_sys_ids() {
    let mut rdr = LexiconReader::new();
    let (splits, rel) = rdr.parse_splits("0/1/2").unwrap();
    assert_eq!(splits.len(), 3);
    assert_eq!(rel, 0);
    assert_eq!(splits[0], SplitUnit::Ref(WordId::new(0, 0)));
    assert_eq!(splits[1], SplitUnit::Ref(WordId::new(0, 1)));
    assert_eq!(splits[2], SplitUnit::Ref(WordId::new(0, 2)));
}

#[test]
fn parse_split_user_ids() {
    let mut rdr = LexiconReader::new();
    let (splits, rel) = rdr.parse_splits("0/U1/2").unwrap();
    assert_eq!(splits.len(), 3);
    assert_eq!(rel, 0);
    assert_eq!(splits[0], SplitUnit::Ref(WordId::new(0, 0)));
    assert_eq!(splits[1], SplitUnit::Ref(WordId::new(1, 1)));
    assert_eq!(splits[2], SplitUnit::Ref(WordId::new(0, 2)));
}

#[test]
fn parse_split_inline() {
    let mut rdr = LexiconReader::new();
    let (splits, rel) = rdr.parse_splits("0/あ,0,1,2,3,4,5,あ/2").unwrap();
    assert_eq!(splits.len(), 3);
    assert_eq!(rel, 1);
    assert_eq!(splits[0], SplitUnit::Ref(WordId::new(0, 0)));
    assert_eq!(
        splits[1],
        SplitUnit::Inline {
            surface: "あ".to_string(),
            pos: 0,
            reading: None
        }
    );
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
    assert_eq!(0, kyoto.synonym_groups.len());
}

#[test]
fn parse_kyoto_not_enough_fields() {
    let mut rdr = LexiconReader::new();
    // last field is omitted
    let data = "京都,1,1,5293,京都,名詞,固有名詞,地名,一般,*,*,キョウト,京都,*,A,*,*";

    assert_matches!(
        rdr.read_bytes(data.as_bytes()),
        Err(SudachiError::DictionaryCompilationError(DicBuildError {
            cause: BuildFailure::NoRawField(_),
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
        Err(SudachiError::DictionaryCompilationError(DicBuildError {
            cause: BuildFailure::EmptySurface,
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
        Err(SudachiError::DictionaryCompilationError(DicBuildError {
            cause: BuildFailure::PosLimitExceeded(_),
            ..
        }))
    );
}

#[test]
fn resolve_inline_same_dict() {
    let mut rdr = DictBuilder::new_system();
    let nread = rdr
        .read_lexicon(include_bytes!("data_kyoto_inline.csv"))
        .unwrap();
    assert_eq!(nread, 3);
    let nresolved = rdr.resolve().unwrap();
    assert_eq!(nresolved, 2);
    let e2 = &rdr.lexicon.entries()[2];
    assert_eq!(e2.splits_a[0], SplitUnit::Ref(WordId::new(0, 1))); //　東
    assert_eq!(e2.splits_a[1], SplitUnit::Ref(WordId::new(0, 0))); // 京都
}

#[test]
fn word_info_rw() {
    let mut rdr = LexiconReader::new();
    let data: &[u8] = include_bytes!("data_kyoto_inline.csv");
    rdr.read_bytes(data).unwrap();
    let mut u16w = Utf16Writer::new();
    let mut data: Vec<u8> = Vec::new();
    rdr.entries[0]
        .write_word_info(&mut u16w, &mut data)
        .unwrap();

    let wi: WordInfo = WordInfoParser::default().parse(&data).unwrap().into();
    assert_eq!(wi.surface(), "京都");
    assert_eq!(wi.dictionary_form(), "京都");
    assert_eq!(wi.normalized_form(), "京都");
    assert_eq!(wi.reading_form(), "キョウト");
    assert_eq!(wi.a_unit_split().len(), 0);
    assert_eq!(wi.b_unit_split().len(), 0);
    assert_eq!(wi.word_structure().len(), 0);
    assert_eq!(wi.synonym_group_ids().len(), 0);
    assert_eq!(wi.dictionary_form_word_id(), -1);
}
