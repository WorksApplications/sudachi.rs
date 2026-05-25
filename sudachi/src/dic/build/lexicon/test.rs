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

use super::*;
use crate::dic::binary_loader::LoadedDictionary;
use crate::dic::build::error::DicBuildError;
use crate::dic::build::DictBuilder;
use crate::dic::description::{Block, Description};
use crate::dic::word_id::{WordId, WordRef as DicWordRef};
use crate::dic::word_info::{WordInfoParser, WordInfos};
use crate::error::SudachiError;
use claim::assert_matches;
use std::fmt::Write;

mod v0;

#[test]
fn parse_split_empty() {
    let mut rdr = LexiconReader::new();
    assert_eq!(rdr.parse_splits("", true).unwrap().len(), 0);
    assert_eq!(rdr.parse_splits("*", true).unwrap().len(), 0);
}

#[test]
fn parse_split_sys_ids() {
    let mut rdr = LexiconReader::new();
    let splits = rdr.parse_splits("0/1/2", true).unwrap();
    assert_eq!(splits.len(), 3);
    assert_eq!(splits[0], WordRef::LineRef(DicWordRef::new(true, 0)));
    assert_eq!(splits[1], WordRef::LineRef(DicWordRef::new(true, 1)));
    assert_eq!(splits[2], WordRef::LineRef(DicWordRef::new(true, 2)));
}

#[test]
fn parse_split_user_ids() {
    let mut rdr = LexiconReader::new();
    let splits = rdr.parse_splits("0/U1/2", true).unwrap();
    assert_eq!(splits.len(), 3);
    assert_eq!(splits[0], WordRef::LineRef(DicWordRef::new(true, 0)));
    assert_eq!(splits[1], WordRef::LineRef(DicWordRef::new(false, 1)));
    assert_eq!(splits[2], WordRef::LineRef(DicWordRef::new(true, 2)));
}

#[test]
fn parse_split_entrykey() {
    let mut rdr = LexiconReader::new();
    let splits = rdr.parse_splits("0/あ,0,1,2,3,4,5,あ/2", true).unwrap();
    assert_eq!(splits.len(), 3);
    assert_eq!(splits[0], WordRef::LineRef(DicWordRef::new(true, 0)));
    assert_eq!(
        splits[1],
        WordRef::EntryKey {
            headword: "あ".to_string(),
            pos: 0,
            reading: "あ".to_string(),
            reference_id: None,
        }
    );
    assert_eq!(splits[2], WordRef::LineRef(DicWordRef::new(true, 2)));
}

#[test]
fn parse_split_entrykey_pos_id() {
    let mut rdr = LexiconReader::new();
    let splits = rdr.parse_splits("0/あ,0,あ/2", true).unwrap();
    assert_eq!(splits.len(), 3);
    assert_eq!(splits[0], WordRef::LineRef(DicWordRef::new(true, 0)));
    assert_eq!(
        splits[1],
        WordRef::EntryKey {
            headword: "あ".to_string(),
            pos: 0,
            reading: "あ".to_string(),
            reference_id: None,
        }
    );
    assert_eq!(splits[2], WordRef::LineRef(DicWordRef::new(true, 2)));
}

#[test]
fn parse_split_entrykey_pos_id_with_reference_id() {
    let mut rdr = LexiconReader::new();
    let splits = rdr.parse_splits("0/あ,0,あ,ref-1/2", true).unwrap();
    assert_eq!(splits.len(), 3);
    assert_eq!(splits[0], WordRef::LineRef(DicWordRef::new(true, 0)));
    assert_eq!(
        splits[1],
        WordRef::EntryKey {
            headword: "あ".to_string(),
            pos: 0,
            reading: "あ".to_string(),
            reference_id: Some("ref-1".to_string()),
        }
    );
    assert_eq!(splits[2], WordRef::LineRef(DicWordRef::new(true, 2)));
}

#[test]
fn parse_split_entrykey_pos_id_with_empty_reading() {
    let mut rdr = LexiconReader::new();
    let splits = rdr.parse_splits("0/あ,0,/2", true).unwrap();
    assert_eq!(splits.len(), 3);
    assert_eq!(splits[0], WordRef::LineRef(DicWordRef::new(true, 0)));
    assert_eq!(
        splits[1],
        WordRef::EntryKey {
            headword: "あ".to_string(),
            pos: 0,
            reading: "".to_string(),
            reference_id: None,
        }
    );
    assert_eq!(splits[2], WordRef::LineRef(DicWordRef::new(true, 2)));
}

#[test]
fn parse_split_entrykey_empty_reference_id_fails() {
    let mut rdr = LexiconReader::new();
    assert_matches!(
        rdr.parse_splits("あ,0,あ,", true),
        Err(BuildFailure::InvalidSplit(_))
    );
}

#[test]
fn parse_split_disallow_numeric_ref() {
    let mut rdr = LexiconReader::new();
    assert_matches!(
        rdr.parse_splits("0/1", false),
        Err(BuildFailure::InvalidSplit(_))
    );
}

#[test]
fn parse_kyoto() {
    let mut rdr = LexiconReader::new();
    let data = concat!(
        "index_form,left_id,right_id,cost,headword,pos1,pos2,pos3,pos4,pos5,pos6,reading_form,normalized_form,dictionary_form,mode,split_a,split_b,word_structure,synonym_groups\n",
        "京都,6,6,5293,京都,名詞,固有名詞,地名,一般,*,*,キョウト,京都,,A,,,,"
    );
    rdr.read_bytes(data.as_bytes()).unwrap();
    let entries = rdr.entries();
    assert_eq!(entries.len(), 1);
    let kyoto = &entries[0];
    assert_eq!("京都", kyoto.index_form);
    assert_eq!(0, kyoto.pos);
    assert_eq!(
        "名詞,固有名詞,地名,一般,*,*",
        format!("{:?}", rdr.pos_obj(kyoto.pos).unwrap())
    );
    assert_eq!(6, kyoto.left_id);
    assert_eq!(6, kyoto.right_id);
    assert_eq!(5293, kyoto.cost);
    assert_eq!("キョウト", kyoto.reading());
    assert_eq!("キョウト", kyoto.reading);
    assert_eq!("京都", kyoto.norm_form());
    assert_eq!(WordRef::SelfRef, kyoto.norm_form);
    assert_eq!(Mode::A, kyoto.splitting);
    assert_eq!(0, kyoto.splits_a.len());
    assert_eq!(0, kyoto.splits_b.len());
    assert!(kyoto.should_index());
}

#[test]
fn parse_header_with_pos_id_only() {
    let mut rdr = LexiconReader::new();
    let data = concat!(
        "index_form,left_id,right_id,cost,headword,pos_id,reading_form,normalized_form,dictionary_form,mode,split_a,split_b,word_structure,synonym_groups\n",
        "京都,6,6,5293,京都,0,キョウト,京都,,A,,,,"
    );
    // preload one POS to resolve pos_id=0
    let old = "京都,6,6,5293,京都,名詞,固有名詞,地名,一般,*,*,キョウト,京都,*,A,*,*,*,*";
    rdr.read_bytes(old.as_bytes()).unwrap();
    let before = rdr.entries().len();
    rdr.read_bytes(data.as_bytes()).unwrap();
    let kyoto = &rdr.entries()[before];
    assert_eq!(kyoto.index_form, "京都");
    assert_eq!(kyoto.pos, 0);
}

#[test]
fn parse_header_word_structure_triple_ref() {
    let mut rdr = LexiconReader::new();
    let data = concat!(
        "index_form,left_id,right_id,cost,pos1,pos2,pos3,pos4,pos5,pos6,reading_form,normalized_form,dictionary_form,mode,split_a,split_b,word_structure\n",
        "東京都,6,8,5320,名詞,固有名詞,地名,一般,*,*,トウキョウト,,,B,\"東京,0,トウキョウ/都,1,ト\",\"東京,0,トウキョウ/都,2,ト\",\"東京,0,トウキョウ/都,3,ト\"\n"
    );
    rdr.read_bytes(data.as_bytes()).unwrap();
    let e = &rdr.entries()[0];
    assert_eq!(e.splits_a.len(), 2);
    assert_eq!(e.splits_b.len(), 2);
    assert_eq!(e.word_structure.len(), 2);
    assert!(matches!(e.word_structure[0], WordRef::EntryKey { .. }));
}

#[test]
fn parse_header_word_structure_triple_ref_with_reference_id() {
    let mut rdr = LexiconReader::new();
    let data = concat!(
        "index_form,left_id,right_id,cost,pos1,pos2,pos3,pos4,pos5,pos6,reading_form,normalized_form,dictionary_form,mode,split_a,split_b,word_structure,reference_id\n",
        "東京都,6,8,5320,名詞,固有名詞,地名,一般,*,*,トウキョウト,,,B,\"東京,0,トウキョウ,ref-1\",\"東京,0,トウキョウ,ref-1\",\"東京,0,トウキョウ,ref-1\",tokyo-metropolis\n"
    );
    rdr.read_bytes(data.as_bytes()).unwrap();
    let e = &rdr.entries()[0];
    assert_eq!(e.reference_id(), Some("tokyo-metropolis"));
    assert_eq!(
        e.word_structure[0],
        WordRef::EntryKey {
            headword: "東京".to_string(),
            pos: 0,
            reading: "トウキョウ".to_string(),
            reference_id: Some("ref-1".to_string()),
        }
    );
}

#[test]
fn parse_header_dictionary_form_asterisk_fails() {
    let mut rdr = LexiconReader::new();
    let data = concat!(
        "index_form,left_id,right_id,cost,pos1,pos2,pos3,pos4,pos5,pos6,reading_form,normalized_form,dictionary_form,mode,split_a,split_b,word_structure\n",
        "東京都,6,8,5320,名詞,固有名詞,地名,一般,*,*,トウキョウト,,*,B,,,\n"
    );
    assert_matches!(
        rdr.read_bytes(data.as_bytes()),
        Err(SudachiError::DictionaryCompilationError(DicBuildError {
            cause: BuildFailure::InvalidSplit(_),
            ..
        }))
    );
}

#[test]
fn parse_header_split_asterisk_fails() {
    let mut rdr = LexiconReader::new();
    let data = concat!(
        "index_form,left_id,right_id,cost,pos1,pos2,pos3,pos4,pos5,pos6,reading_form,normalized_form,dictionary_form,mode,split_a,split_b,word_structure\n",
        "東京都,6,8,5320,名詞,固有名詞,地名,一般,*,*,トウキョウト,,,B,*,,\n"
    );
    assert_matches!(
        rdr.read_bytes(data.as_bytes()),
        Err(SudachiError::DictionaryCompilationError(DicBuildError {
            cause: BuildFailure::InvalidSplit(_),
            ..
        }))
    );
}

#[test]
fn parse_header_synonym_groups_asterisk_fails() {
    let mut rdr = LexiconReader::new();
    let data = concat!(
        "index_form,left_id,right_id,cost,headword,pos_id,reading_form,normalized_form,dictionary_form,mode,split_a,split_b,word_structure,synonym_groups\n",
        "京都,6,6,5293,京都,0,キョウト,京都,,A,,,,*"
    );
    // preload one POS to resolve pos_id=0
    let old = "京都,6,6,5293,京都,名詞,固有名詞,地名,一般,*,*,キョウト,京都,*,A,*,*,*,*";
    rdr.read_bytes(old.as_bytes()).unwrap();
    assert_matches!(
        rdr.read_bytes(data.as_bytes()),
        Err(SudachiError::DictionaryCompilationError(DicBuildError {
            cause: BuildFailure::InvalidSplit(_),
            ..
        }))
    );
}

#[test]
fn resolve_header_normalized_form_ref() {
    let mut bldr = DictBuilder::new_system();
    let data = concat!(
        "index_form,left_id,right_id,cost,pos1,pos2,pos3,pos4,pos5,pos6,reading_form,normalized_form,dictionary_form,mode,split_a,split_b,word_structure\n",
        "東京,1,1,2816,名詞,固有名詞,地名,一般,*,*,トウキョウ,,,A,,,\n",
        "トウキョウ,1,1,2816,名詞,固有名詞,地名,一般,*,*,トウキョウ,\"東京,0,トウキョウ\",,A,,,\n"
    );
    bldr.read_lexicon(data.as_bytes()).unwrap();
    bldr.resolve().unwrap();
    let refs = bldr.lexicon.row_word_refs(false);
    let e = &bldr.lexicon.resolved_entries()[1];
    assert_eq!(e.norm_form, ResolvedWordRef::Ref(refs[0]));
}

#[test]
fn resolve_header_normalized_form_headword_ref() {
    let mut bldr = DictBuilder::new_system();
    let data = concat!(
        "index_form,left_id,right_id,cost,pos1,pos2,pos3,pos4,pos5,pos6,reading_form,normalized_form,dictionary_form,mode,split_a,split_b,word_structure\n",
        "東京,1,1,2816,名詞,固有名詞,地名,一般,*,*,トウキョウ,,,A,,,\n",
        "トーキョー,1,1,2816,名詞,固有名詞,地名,一般,*,*,トーキョー,東京,,A,,,\n"
    );
    bldr.read_lexicon(data.as_bytes()).unwrap();
    bldr.resolve().unwrap();
    let refs = bldr.lexicon.row_word_refs(false);
    let e = &bldr.lexicon.resolved_entries()[1];
    assert_eq!(e.norm_form, ResolvedWordRef::Ref(refs[0]));
}

#[test]
fn parse_dictionary_form_entrykey_self_reference_is_not_rewritten_to_selfref() {
    let mut rdr = LexiconReader::new();
    let data = concat!(
        "index_form,left_id,right_id,cost,headword,pos1,pos2,pos3,pos4,pos5,pos6,reading_form,normalized_form,dictionary_form,mode,split_a,split_b,word_structure\n",
        "京都,1,1,100,京都,名詞,固有名詞,地名,一般,*,*,キョウト,,,A,,,\n",
        "京都,1,1,100,京都,名詞,固有名詞,地名,一般,*,*,キョウト,,\"京都,名詞,固有名詞,地名,一般,*,*,キョウト\",A,,,\n"
    );
    rdr.read_bytes(data.as_bytes()).unwrap();

    assert_eq!(
        rdr.entries()[1].dic_form,
        WordRef::EntryKey {
            headword: "京都".to_string(),
            pos: 0,
            reading: "キョウト".to_string(),
            reference_id: None,
        }
    );
}

#[test]
fn read_pos_table_and_parse_pos_id_lexicon() {
    let mut rdr = LexiconReader::new();
    let pos = "0,名詞,固有名詞,地名,一般,*,*\n1,名詞,一般,*,*,*,*";
    assert_eq!(rdr.read_pos_bytes(pos.as_bytes()).unwrap(), 2);

    let lex = concat!(
        "index_form,left_id,right_id,cost,headword,pos_id,reading_form,normalized_form,dictionary_form,mode,split_a,split_b,word_structure,synonym_groups\n",
        "京都,6,6,5293,京都,0,キョウト,京都,,A,,,,"
    );
    rdr.read_bytes(lex.as_bytes()).unwrap();
    let e = &rdr.entries()[0];
    assert_eq!(e.pos, 0);
}

#[test]
fn read_pos_table_header_without_pos_id() {
    let mut rdr = LexiconReader::new();
    let pos = "pos1,pos2,pos3,pos4,pos5,pos6\n名詞,固有名詞,地名,一般,*,*";
    assert_eq!(rdr.read_pos_bytes(pos.as_bytes()).unwrap(), 1);
    assert_eq!(
        format!("{:?}", rdr.pos_obj(0).unwrap()),
        "名詞,固有名詞,地名,一般,*,*"
    );
}

#[test]
fn read_pos_table_unordered_columns_without_pos_id() {
    let mut rdr = LexiconReader::new();
    let pos = "pos5,pos6,pos1,pos2,pos3,pos4\n*,*,名詞,固有名詞,地名,一般";
    assert_eq!(rdr.read_pos_bytes(pos.as_bytes()).unwrap(), 1);
    assert_eq!(
        format!("{:?}", rdr.pos_obj(0).unwrap()),
        "名詞,固有名詞,地名,一般,*,*"
    );
}

#[test]
fn read_pos_table_unordered_pos_ids() {
    let mut rdr = LexiconReader::new();
    let pos = concat!(
        "pos_id,pos1,pos2,pos3,pos4,pos5,pos6\n",
        "1,名詞,普通名詞,一般,*,*,*\n",
        "0,助詞,接続助詞,*,*,*,*"
    );
    assert_eq!(rdr.read_pos_bytes(pos.as_bytes()).unwrap(), 2);
    assert_eq!(
        format!("{:?}", rdr.pos_obj(0).unwrap()),
        "助詞,接続助詞,*,*,*,*"
    );
    assert_eq!(
        format!("{:?}", rdr.pos_obj(1).unwrap()),
        "名詞,普通名詞,一般,*,*,*"
    );
}

#[test]
fn read_pos_table_non_header_invalid_pos_id_literal() {
    let mut rdr = LexiconReader::new();
    let pos = "40000,名詞,固有名詞,地名,一般,*,*";
    assert_matches!(
        rdr.read_pos_bytes(pos.as_bytes()),
        Err(SudachiError::DictionaryCompilationError(DicBuildError {
            cause: BuildFailure::InvalidI16Literal(v),
            line: 1,
            ..
        })) if v == "40000"
    );
}

#[test]
fn read_pos_table_non_contiguous_id_fails() {
    let mut rdr = LexiconReader::new();
    let pos = "2,名詞,固有名詞,地名,一般,*,*";
    assert_matches!(
        rdr.read_pos_bytes(pos.as_bytes()),
        Err(SudachiError::DictionaryCompilationError(DicBuildError {
            cause: BuildFailure::InvalidSplit(_),
            ..
        }))
    );
}

#[test]
fn parse_header_reads_split_c_and_user_data() {
    let mut rdr = LexiconReader::new();
    let data = concat!(
        "index_form,left_id,right_id,cost,pos1,pos2,pos3,pos4,pos5,pos6,reading_form,normalized_form,dictionary_form,mode,split_a,split_b,word_structure,split_c,user_data\n",
        "東京都,6,8,5320,名詞,固有名詞,地名,一般,*,*,トウキョウト,,,C,\"東京,0,トウキョウ/都,1,ト\",\"東京,0,トウキョウ/都,2,ト\",\"東京,0,トウキョウ/都,3,ト\",\"東京,0,トウキョウ/都,4,ト\",meta"
    );
    rdr.read_bytes(data.as_bytes()).unwrap();
    let e = &rdr.entries()[0];
    assert_eq!(e.splits_c.len(), 2);
    assert_eq!(e.user_data, "meta");
}

#[test]
fn parse_header_reference_id_empty_is_none() {
    let mut rdr = LexiconReader::new();
    let data = concat!(
        "index_form,left_id,right_id,cost,headword,pos_id,reading_form,normalized_form,dictionary_form,mode,split_a,split_b,word_structure,synonym_groups,reference_id\n",
        "京都,6,6,5293,京都,0,キョウト,京都,,A,,,,,\n"
    );
    let old = "京都,6,6,5293,京都,名詞,固有名詞,地名,一般,*,*,キョウト,京都,*,A,*,*,*,*";
    rdr.read_bytes(old.as_bytes()).unwrap();
    let before = rdr.entries().len();
    rdr.read_bytes(data.as_bytes()).unwrap();
    assert_eq!(rdr.entries()[before].reference_id(), None);
}

#[test]
fn parse_header_user_data_multibyte_within_char_limit() {
    let mut rdr = LexiconReader::new();
    let user_data = "あ".repeat(11_000);
    let data = format!(
        concat!(
            "index_form,left_id,right_id,cost,pos1,pos2,pos3,pos4,pos5,pos6,reading_form,normalized_form,dictionary_form,mode,split_a,split_b,word_structure,user_data\n",
            "京都,6,6,5293,名詞,固有名詞,地名,一般,*,*,キョウト,京都,,A,,,,{}"
        ),
        user_data
    );
    rdr.read_bytes(data.as_bytes()).unwrap();
    assert_eq!(rdr.entries()[0].user_data, user_data);
}

#[test]
fn parse_header_mode_optional() {
    let mut rdr = LexiconReader::new();
    let data = concat!(
        "index_form,left_id,right_id,cost,pos1,pos2,pos3,pos4,pos5,pos6,reading_form,normalized_form,dictionary_form,split_a,split_b,word_structure\n",
        "京都,6,6,5293,名詞,固有名詞,地名,一般,*,*,キョウト,京都,,,,"
    );
    rdr.read_bytes(data.as_bytes()).unwrap();
    let e = &rdr.entries()[0];
    assert_eq!(e.splitting, Mode::C);
}

#[test]
fn resolve_header_normalized_form_literal_without_target() {
    let mut bldr = DictBuilder::new_system();
    let data = concat!(
        "index_form,left_id,right_id,cost,pos1,pos2,pos3,pos4,pos5,pos6,reading_form,normalized_form,dictionary_form,mode,split_a,split_b,word_structure\n",
        "舞台藝術,1,1,2816,名詞,普通名詞,一般,*,*,*,ブタイゲイジュツ,舞台芸術,,A,,,"
    );
    bldr.read_lexicon(data.as_bytes()).unwrap();
    bldr.resolve().unwrap();
    assert_eq!(bldr.lexicon.entries().len(), 1);
    assert_eq!(bldr.lexicon.resolved_entries().len(), 2);
    let e = &bldr.lexicon.resolved_entries()[0];
    assert_eq!(
        e.norm_form,
        ResolvedWordRef::Ref(DicWordRef::new(true, bldr.lexicon.next_entry_id()))
    );
    let phantom = &bldr.lexicon.resolved_entries()[1];
    assert_eq!(phantom.headword(), "舞台芸術");
    assert!(!phantom.should_index());
}

#[test]
fn parse_normalized_form_self_reference_respects_reference_id() {
    let mut rdr = LexiconReader::new();
    let data = concat!(
        "index_form,left_id,right_id,cost,headword,pos1,pos2,pos3,pos4,pos5,pos6,reading_form,normalized_form,dictionary_form,mode,split_a,split_b,word_structure,reference_id\n",
        "京都,1,1,100,京都,名詞,固有名詞,地名,一般,*,*,キョウト,\"京都,0,キョウト,kyoto-1\",,A,,,,kyoto-1\n",
        "京都,1,1,100,京都,名詞,固有名詞,地名,一般,*,*,キョウト,\"京都,0,キョウト,kyoto-1\",,A,,,,kyoto-2\n"
    );
    rdr.read_bytes(data.as_bytes()).unwrap();

    assert_eq!(rdr.entries()[0].norm_form, WordRef::SelfRef);
    assert_eq!(
        rdr.entries()[1].norm_form,
        WordRef::EntryKey {
            headword: "京都".to_string(),
            pos: 0,
            reading: "キョウト".to_string(),
            reference_id: Some("kyoto-1".to_string()),
        }
    );
}

#[test]
fn parse_kyoto_ignored() {
    let mut rdr = LexiconReader::new();
    let data = concat!(
        "index_form,left_id,right_id,cost,headword,pos1,pos2,pos3,pos4,pos5,pos6,reading_form,normalized_form,dictionary_form,mode,split_a,split_b,word_structure,synonym_groups\n",
        "京都,-1,-1,5293,京都,名詞,固有名詞,地名,一般,*,*,キョウト,京都,,A,,,,"
    );
    rdr.read_bytes(data.as_bytes()).unwrap();
    let entries = rdr.entries();
    assert_eq!(entries.len(), 1);
    let kyoto = &entries[0];
    assert!(!kyoto.should_index());
}

#[test]
fn parse_kyoto_synonym_opt() {
    let mut rdr = LexiconReader::new();
    // synonym_groups column itself is omitted
    let data = concat!(
        "index_form,left_id,right_id,cost,headword,pos1,pos2,pos3,pos4,pos5,pos6,reading_form,normalized_form,dictionary_form,mode,split_a,split_b,word_structure\n",
        "京都,1,1,5293,京都,名詞,固有名詞,地名,一般,*,*,キョウト,京都,,A,,,"
    );
    rdr.read_bytes(data.as_bytes()).unwrap();
    let entries = rdr.entries();
    assert_eq!(entries.len(), 1);
    let kyoto = &entries[0];
    assert_eq!(0, kyoto.synonym_groups.len());
}

#[test]
fn parse_kyoto_not_enough_fields() {
    let mut rdr = LexiconReader::new();
    // word_structure and synonym_groups are missing
    let data = concat!(
        "index_form,left_id,right_id,cost,headword,pos1,pos2,pos3,pos4,pos5,pos6,reading_form,normalized_form,dictionary_form,mode,split_a,split_b,word_structure,synonym_groups\n",
        "京都,1,1,5293,京都,名詞,固有名詞,地名,一般,*,*,キョウト,京都,,A,,"
    );

    assert_matches!(
        rdr.read_bytes(data.as_bytes()),
        Err(SudachiError::DictionaryCompilationError(DicBuildError {
            cause: BuildFailure::NoRawField(_),
            line: 2,
            ..
        }))
    );
}

#[test]
fn parse_kyoto_ignored_empty_index_form() {
    let mut rdr = LexiconReader::new();
    let data = concat!(
        "index_form,left_id,right_id,cost,headword,pos1,pos2,pos3,pos4,pos5,pos6,reading_form,normalized_form,dictionary_form,mode,split_a,split_b,word_structure,synonym_groups\n",
        ",-1,-1,5293,京都,名詞,固有名詞,地名,一般,*,*,キョウト,京都,,A,,,,"
    );
    assert_matches!(
        rdr.read_bytes(data.as_bytes()),
        Err(SudachiError::DictionaryCompilationError(DicBuildError {
            cause: BuildFailure::EmptyIndexForm,
            line: 2,
            ..
        }))
    );
}

#[test]
fn parse_pos_exhausted() {
    let mut rdr = LexiconReader::new();
    let mut data = String::from(
        "index_form,left_id,right_id,cost,headword,pos1,pos2,pos3,pos4,pos5,pos6,reading_form,normalized_form,dictionary_form,mode,split_a,split_b,word_structure,synonym_groups\n",
    );
    for i in 0..=MAX_POS_IDS + 1 {
        writeln!(
            data,
            "x,-1,-1,5293,京都,名詞,固有名詞,地名,一般,*,{},キョウト,京都,,A,,,,",
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
fn resolve_entrykey_same_dict() {
    let mut rdr = DictBuilder::new_system();
    let nread = rdr
        .read_lexicon(include_bytes!("data_kyoto_entrykey.csv"))
        .unwrap();
    assert_eq!(nread, 3);
    let nresolved = rdr.resolve().unwrap();
    assert_eq!(nresolved, 2);
    let e2 = &rdr.lexicon.resolved_entries()[2];
    assert_eq!(e2.splits_a[0], DicWordRef::new(true, 10)); // 東
    assert_eq!(e2.splits_a[1], DicWordRef::new(true, 4)); // 京都
}

#[test]
fn word_info_rw() {
    let mut bldr = DictBuilder::new_system();
    bldr.read_conn(include_bytes!("../test/matrix_10x10.def"))
        .unwrap();
    bldr.read_lexicon(include_bytes!("data_kyoto_entrykey.csv"))
        .unwrap();
    bldr.resolve().unwrap();

    let mut bin = Vec::new();
    bldr.compile(&mut bin).unwrap();
    let dic = LoadedDictionary::load_system(&bin).unwrap();
    let target = dic
        .lexicon_set
        .lookup("京都".as_bytes(), 0)
        .next()
        .unwrap()
        .word_id;
    let desc = Description::load(&bin).unwrap();
    let entries = desc.slice(&bin, Block::Entries).unwrap();
    let offset = (target.entry().as_raw() as usize) << WordInfos::WORD_ID_ALIGNMENT_BITS;

    let wi = WordInfoParser::default().parse(&entries[offset..]).unwrap();
    assert_eq!(wi.pos_id, 0);
    assert!(wi.headword_strptr.length > 0);
    assert!(wi.reading_form_strptr.length > 0);
    assert_ne!(wi.normalized_form, WordId::INVALID.as_raw());
    assert_ne!(wi.dictionary_form, WordId::INVALID.as_raw());
    assert_eq!(wi.index_form_length, "京都".len() as i16);
    assert_eq!(wi.c_unit_split.len(), 0);
    assert_eq!(wi.b_unit_split.len(), 0);
    assert_eq!(wi.a_unit_split.len(), 0);
    assert_eq!(wi.word_structure.len(), 0);
    assert_eq!(wi.synonym_group_ids.len(), 0);
    assert_eq!(wi.user_data, "user");
}
