/*
 * Copyright (c) 2021-2026 Works Applications Co., Ltd.
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

use std::ops::Deref;
use std::time::{Duration, UNIX_EPOCH};

mod common;
use common::{TestTokenizer, GRAMMAR};
use sudachi::analysis::morpheme::MorphemeView;
use sudachi::dic::build::DictBuilder;
use sudachi::dic::dictionary::JapaneseDictionary;
use sudachi::dic::error::DictionaryCompatibilityError;
use sudachi::dic::storage::{Storage, SudachiDicData};
use sudachi::error::SudachiError;

const NON_INDEXED_ENTRY_LEXICON: &[u8] = concat!(
    "index_form,left_id,right_id,cost,pos1,pos2,pos3,pos4,pos5,pos6,reading_form,normalized_form,dictionary_form,mode,split_a,split_b,word_structure\n",
    "京都,6,6,5293,名詞,固有名詞,地名,一般,*,*,キョウト,,,A,,,\n",
    "隠し,-1,-1,5293,名詞,普通名詞,一般,*,*,*,カクシ,,,A,,,\n",
    "舞台藝術,1,1,2816,名詞,普通名詞,一般,*,*,*,ブタイゲイジュツ,舞台芸術,,A,,,\n",
)
.as_bytes();

const NON_INDEXED_ENTRY_CONFIG: &str = r#"
{
    "path" : "tests/resources/",
    "characterDefinitionFile" : "char.def",
    "inputTextPlugin" : [
        { "class" : "$exe/default_input_text" }
    ],
    "oovProviderPlugin" : [
        { "class" : "$exe/simple_oov",
          "oovPOS" : [ "名詞", "普通名詞", "一般", "*", "*", "*" ],
          "leftId" : 8,
          "rightId" : 8,
          "cost" : 6000 }
    ],
    "pathRewritePlugin" : []
}
"#;

const YOMIGANA_CONFIG: &str = r#"
{
    "path" : "tests/resources/",
    "characterDefinitionFile" : "char.def",
    "inputTextPlugin" : [
        { "class" : "com.worksap.nlp.sudachi.DefaultInputTextPlugin" },
        { "class" : "com.worksap.nlp.sudachi.IgnoreYomiganaPlugin",
          "leftBrackets": ["(", "（"],
          "rightBrackets": [")", "）"],
          "maxYomiganaLength": 4 }
    ],
    "oovProviderPlugin" : [
        { "class" : "com.worksap.nlp.sudachi.SimpleOovPlugin",
          "oovPOS" : [ "名詞", "普通名詞", "一般", "*", "*", "*" ],
          "leftId" : 8,
          "rightId" : 8,
          "cost" : 6000 }
    ],
    "pathRewritePlugin" : []
}
"#;

#[test]
fn get_part_of_speech_size() {
    // pos from system test dict
    assert_eq!(14, GRAMMAR.pos_list.len());

    // user test dict contains another pos
    let tokenizer = TestTokenizer::new();
    assert_eq!(15, tokenizer.dict().grammar().pos_list.len());
}

#[test]
fn get_part_of_speech_string() {
    let pos = &GRAMMAR.pos_list[0];
    assert!(!pos.is_empty());
    assert_eq!("助動詞", pos[0]);
}

#[test]
fn reject_incompatible_user_dictionary() {
    let mut another_system = DictBuilder::new_system();
    another_system.set_compile_time(UNIX_EPOCH + Duration::from_secs(1));
    another_system.set_description("another");
    another_system
        .read_conn(include_bytes!("resources/matrix_10x10.def"))
        .unwrap();
    another_system.read_lexicon(common::LEX_CSV).unwrap();
    another_system.resolve().unwrap();

    let mut another_system_bytes = Vec::new();
    another_system.compile(&mut another_system_bytes).unwrap();
    let another_system_dic =
        sudachi::dic::binary_loader::LoadedDictionary::load_system(&another_system_bytes).unwrap();
    let another_system_desc =
        sudachi::dic::binary_loader::BinaryDictionary::load_system(&another_system_bytes)
            .unwrap()
            .description;

    let mut user = DictBuilder::new_user(&another_system_dic);
    user.read_lexicon(common::USER1_CSV).unwrap();
    user.resolve().unwrap();

    let mut system = DictBuilder::new_system();
    system.set_compile_time(UNIX_EPOCH + Duration::from_secs(2));
    system.set_description("system");
    system
        .read_conn(include_bytes!("resources/matrix_10x10.def"))
        .unwrap();
    system.read_lexicon(common::LEX_CSV).unwrap();
    system.resolve().unwrap();

    let mut system_bytes = Vec::new();
    system.compile(&mut system_bytes).unwrap();
    let system_desc = sudachi::dic::binary_loader::BinaryDictionary::load_system(&system_bytes)
        .unwrap()
        .description;

    let mut user_bytes = Vec::new();
    user.compile(&mut user_bytes).unwrap();

    let mut storage = SudachiDicData::new(Storage::Owned(system_bytes));
    storage.add_user(Storage::Owned(user_bytes));

    match JapaneseDictionary::from_cfg_storage(&common::TEST_CONFIG, storage) {
        Err(SudachiError::DictionaryCompatibility(
            DictionaryCompatibilityError::UserDictionary {
                user_index,
                system_signature,
                user_reference,
            },
        )) => {
            assert_eq!(user_index, 0);
            assert_eq!(system_signature, system_desc.signature());
            assert_eq!(user_reference, another_system_desc.signature());
        }
        Ok(_) => {
            panic!("dictionary creation should have failed for an incompatible user dictionary")
        }
        Err(err) => panic!("unexpected error: {err}"),
    }
}

#[test]
fn entries_include_non_indexed_entries_and_exclude_phantoms() {
    let tok = common::TestStatefulTokenizer::builder(NON_INDEXED_ENTRY_LEXICON)
        .config(NON_INDEXED_ENTRY_CONFIG.as_bytes())
        .build();

    let entries = tok
        .dict()
        .entries()
        .map(|entry| entry.unwrap().surface().to_owned())
        .collect::<Vec<_>>();

    assert_eq!(entries.len(), 3);
    assert!(entries.contains(&"京都".to_string()));
    assert!(entries.contains(&"隠し".to_string()));
    assert!(entries.contains(&"舞台藝術".to_string()));
}

#[test]
fn lookup_all_entries_scans_non_indexed_entries_only_public_rows() {
    let mut tok = common::TestStatefulTokenizer::builder(NON_INDEXED_ENTRY_LEXICON)
        .config(NON_INDEXED_ENTRY_CONFIG.as_bytes())
        .build();

    let indexed = tok.dict().lookup_all_entries("京都").unwrap();
    assert_eq!(indexed.len(), 1);
    assert_eq!(indexed[0].reading_form(), "キョウト");

    let non_indexed = tok.dict().lookup_all_entries("隠し").unwrap();
    assert_eq!(non_indexed.len(), 1);
    assert_eq!(non_indexed[0].surface(), "隠し");

    tok.result.clear();
    assert_eq!(
        tok.result
            .lookup("隠し", sudachi::dic::subset::InfoSubset::all())
            .unwrap(),
        0
    );

    let phantom = tok.dict().lookup_all_entries("舞台芸術").unwrap();
    assert!(phantom.is_empty());
}

#[test]
fn indexed_lookup_normalizes_query() {
    let mut tok = common::TestStatefulTokenizer::new_built(sudachi::analysis::Mode::C);

    assert_eq!(
        tok.result
            .lookup("特A", sudachi::dic::subset::InfoSubset::all())
            .unwrap(),
        1
    );
    assert_eq!(tok.result.get(0).reading_form(), "トクエー");
}

#[test]
fn lookup_all_entries_normalizes_query_and_searches_user_dictionaries() {
    let tok = TestTokenizer::new();

    let normalized = tok.dict().lookup_all_entries("特A").unwrap();
    assert_eq!(normalized.len(), 1);
    assert_eq!(normalized[0].surface(), "特A");

    let user_entry = tok.dict().lookup_all_entries("すだち").unwrap();
    assert_eq!(user_entry.len(), 1);
    assert_eq!(user_entry[0].dictionary_id(), 1);
    assert_eq!(user_entry[0].user_data(), "徳島県産");

    assert!(tok
        .dict()
        .lookup_all_entries("存在しない語")
        .unwrap()
        .is_empty());
}

#[test]
fn lookup_all_entries_and_lookup_apply_input_text_plugins() {
    let mut tok = common::TestStatefulTokenizer::builder(NON_INDEXED_ENTRY_LEXICON)
        .config(YOMIGANA_CONFIG.as_bytes())
        .build();

    let entries = tok.dict().lookup_all_entries("京都（キョウト）").unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].surface(), "京都");
    assert_eq!(entries[0].normalized_form(), "京都");

    tok.result.clear();
    assert_eq!(
        tok.result
            .lookup("京都（キョウト）", sudachi::dic::subset::InfoSubset::all())
            .unwrap(),
        1
    );
    assert_eq!(tok.result.get(0).reading_form(), "キョウト");
    assert_eq!(tok.result.get(0).surface().deref(), "京都");
    assert_eq!(tok.result.get(0).begin(), 0);
    assert_eq!(tok.result.get(0).end(), "京都".len());
    assert_eq!(tok.result.get(0).end_c(), 2);
}

#[test]
fn oov_morpheme_creates_standalone_oov_entry() {
    let tok = TestTokenizer::new();

    let pos_id1 = 1;
    let m1 = tok.dict().oov_morpheme(pos_id1, "OOV").unwrap();
    assert_eq!(m1.begin(), 0);
    assert_eq!(m1.end(), 3);
    assert_eq!(m1.begin_c(), 0);
    assert_eq!(m1.end_c(), 3);
    assert_eq!(m1.part_of_speech_id(), pos_id1);
    assert_eq!(m1.surface(), "OOV");
    assert_eq!(m1.reading_form(), "OOV");
    assert_eq!(m1.normalized_form(), "OOV");
    assert_eq!(m1.dictionary_form(), "OOV");
    assert!(m1.is_oov());
    assert_eq!(
        m1.word_id(),
        sudachi::dic::word_id::WordId::oov(pos_id1 as u32)
    );
    assert_eq!(m1.dictionary_id(), -1);
    assert!(m1.synonym_group_ids().is_empty());
    assert_eq!(m1.user_data(), "");

    let pos_id2 = 2;
    let m2 = tok
        .dict()
        .oov_morpheme_with_forms(pos_id2, "OOVs", "OOVr", "OOVn", "OOVd")
        .unwrap();
    assert_eq!(m2.begin(), 0);
    assert_eq!(m2.end(), 4);
    assert_eq!(m2.part_of_speech_id(), pos_id2);
    assert_eq!(m2.surface(), "OOVs");
    assert_eq!(m2.reading_form(), "OOVr");
    assert_eq!(m2.normalized_form(), "OOVn");
    assert_eq!(m2.dictionary_form(), "OOVd");

    // form_morpheme returns self for OOV morphemes
    let m1_df = m1.dictionary_form_morpheme().unwrap();
    let m1_nf = m1.normalized_form_morpheme().unwrap();
    assert_eq!(m1_df.surface().as_ref(), "OOV");
    assert_eq!(m1_nf.surface().as_ref(), "OOV");
    assert!(m1_df.is_oov());
    assert!(m1_nf.is_oov());

    let m2_df = m2.dictionary_form_morpheme().unwrap();
    let m2_nf = m2.normalized_form_morpheme().unwrap();
    assert_eq!(m2_df.surface().as_ref(), "OOVs");
    assert_eq!(m2_nf.surface().as_ref(), "OOVs");
    assert!(m2_df.is_oov());
    assert!(m2_nf.is_oov());
}

// fn creat_with_merging_settings
// fn creat_with_merging_null_settings
