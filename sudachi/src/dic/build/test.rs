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

use std::time::{Duration, UNIX_EPOCH};

mod legacy;
mod with_analysis;

use crate::dic::binary_loader::{BinaryDictionary, LoadedDictionary};
use crate::dic::build::error::{BuildFailure, DicBuildError};
use crate::dic::build::DictBuilder;
use crate::dic::word_id::WordRef as DicWordRef;
use crate::dic::LexiconAccess;
use crate::error::SudachiError;

static MATRIX_10_10: &[u8] = include_bytes!("test/matrix_10x10.def");
static WORDREF_SYSTEM: &[u8] = include_bytes!("test/wordref.csv");
static WORDREF_USER: &[u8] = include_bytes!("test/wordref-user.csv");

#[test]
fn read_pos_then_read_lexicon_with_pos_id() {
    let mut bldr = DictBuilder::new_system();
    bldr.read_conn(MATRIX_10_10).unwrap();
    let pos = "0,名詞,固有名詞,地名,一般,*,*\n1,名詞,一般,*,*,*,*";
    bldr.read_pos(pos.as_bytes()).unwrap();

    let lex = concat!(
        "index_form,left_id,right_id,cost,headword,pos_id,reading_form,normalized_form,dictionary_form,mode,split_a,split_b,word_structure,synonym_groups\n",
        "京都,6,6,5293,京都,0,キョウト,京都,,A,,,,"
    );
    assert_eq!(1, bldr.read_lexicon(lex.as_bytes()).unwrap());
    bldr.resolve().unwrap();
    let mut out = Vec::new();
    bldr.compile(&mut out).unwrap();
}

#[test]
fn read_pos_then_conn_then_read_lexicon_with_pos_id() {
    let mut bldr = DictBuilder::new_system();
    let pos = "0,名詞,固有名詞,地名,一般,*,*\n1,名詞,一般,*,*,*,*";
    bldr.read_pos(pos.as_bytes()).unwrap();
    bldr.read_conn(MATRIX_10_10).unwrap();

    let lex = concat!(
        "index_form,left_id,right_id,cost,headword,pos_id,reading_form,normalized_form,dictionary_form,mode,split_a,split_b,word_structure,synonym_groups\n",
        "京都,6,6,5293,京都,0,キョウト,京都,,A,,,,"
    );
    assert_eq!(1, bldr.read_lexicon(lex.as_bytes()).unwrap());
    bldr.resolve().unwrap();
    let mut out = Vec::new();
    bldr.compile(&mut out).unwrap();
}

#[test]
fn read_pos_after_lexicon_fails() {
    let mut bldr = DictBuilder::new_system();
    bldr.read_conn(MATRIX_10_10).unwrap();
    bldr.read_lexicon(include_bytes!("test/data_1word.csv"))
        .unwrap();
    let pos = "0,名詞,固有名詞,地名,一般,*,*";
    claim::assert_matches!(
        bldr.read_pos(pos.as_bytes()),
        Err(SudachiError::DictionaryCompilationError(DicBuildError {
            cause: BuildFailure::InvalidBuilderState(_),
            ..
        }))
    );
}

#[test]
fn build_grammar() {
    let mut bldr = DictBuilder::new_system();
    bldr.read_conn(MATRIX_10_10).unwrap();
    assert_eq!(
        1,
        bldr.read_lexicon(include_bytes!("test/data_1word.csv"))
            .unwrap()
    );
    bldr.resolve().unwrap();
    let mut built = Vec::new();
    bldr.compile(&mut built).unwrap();
    let dic = LoadedDictionary::load_system(&built).unwrap();
    let grammar = &dic.grammar;
    assert_eq!(grammar.pos_list.len(), 1);
    assert_eq!(
        grammar.pos_list[0],
        &["名詞", "固有名詞", "地名", "一般", "*", "*"]
    );
    let conn = grammar.conn_matrix();
    assert_eq!(conn.num_left(), 10);
    assert_eq!(conn.num_right(), 10);
}

#[test]
fn build_lexicon_1word() {
    let mut bldr = DictBuilder::new_system();
    bldr.read_conn(MATRIX_10_10).unwrap();
    assert_eq!(
        1,
        bldr.read_lexicon(include_bytes!("test/data_1word.csv"))
            .unwrap()
    );
    bldr.resolve().unwrap();
    let mut built = Vec::new();
    bldr.compile(&mut built).unwrap();
    let dic = LoadedDictionary::load_system(&built).unwrap();
    let mut iter = dic.lexicon().lookup("京都".as_bytes(), 0);
    let entry = iter.next().unwrap();
    assert_eq!(entry.end, 6);
    assert_eq!(entry.word_id.dict().as_raw(), 0);
    assert_eq!(iter.next(), None);
    assert_eq!((6, 6, 5293), dic.lexicon().get_word_param(entry.word_id));
    let wi = dic.lexicon().get_word_info(entry.word_id).unwrap();
    assert_eq!(wi.headword(&dic), "京都");
    assert_eq!(wi.normalized_form(&dic), "京都");
    assert_eq!(wi.dictionary_form(&dic), "京都");
    assert_eq!(wi.reading_form(&dic), "キョウト");
}

#[test]
fn omitted_headword_resolves_normalized_and_dictionary_form_to_self() {
    let mut bldr = DictBuilder::new_system();
    bldr.read_conn(MATRIX_10_10).unwrap();
    let lex = concat!(
        "index_form,left_id,right_id,cost,headword,pos1,pos2,pos3,pos4,pos5,pos6,reading_form,normalized_form,dictionary_form,split_a,split_b,split_c,word_structure,synonym_groups\n",
        "京都,6,6,5293,,名詞,固有名詞,地名,一般,*,*,キョウト,,,,,,,\n"
    );
    assert_eq!(1, bldr.read_lexicon(lex.as_bytes()).unwrap());
    bldr.resolve().unwrap();
    let mut built = Vec::new();
    bldr.compile(&mut built).unwrap();
    let dic = LoadedDictionary::load_system(&built).unwrap();
    let entry = dic.lexicon().lookup("京都".as_bytes(), 0).next().unwrap();
    let wi = dic.lexicon().get_word_info(entry.word_id).unwrap();
    assert_eq!(wi.headword(&dic), "京都");
    assert_eq!(wi.normalized_form(&dic), "京都");
    assert_eq!(wi.dictionary_form(&dic), "京都");
}

#[test]
fn different_headword_resolves_normalized_and_dictionary_form_to_headword() {
    let mut bldr = DictBuilder::new_system();
    bldr.read_conn(MATRIX_10_10).unwrap();
    let lex = concat!(
        "index_form,left_id,right_id,cost,headword,pos1,pos2,pos3,pos4,pos5,pos6,reading_form,normalized_form,dictionary_form,split_a,split_b,split_c,word_structure,synonym_groups\n",
        "東京,6,6,5293,京都,名詞,固有名詞,地名,一般,*,*,トウキョウ,,,,,,,\n"
    );
    assert_eq!(1, bldr.read_lexicon(lex.as_bytes()).unwrap());
    bldr.resolve().unwrap();
    let mut built = Vec::new();
    bldr.compile(&mut built).unwrap();
    let dic = LoadedDictionary::load_system(&built).unwrap();
    let entry = dic.lexicon().lookup("東京".as_bytes(), 0).next().unwrap();
    let wi = dic.lexicon().get_word_info(entry.word_id).unwrap();
    assert_eq!(wi.headword(&dic), "京都");
    assert_eq!(wi.normalized_form(&dic), "京都");
    assert_eq!(wi.dictionary_form(&dic), "京都");
}

#[test]
fn dictionary_form_entrykey_self_reference_resolves_to_previous_duplicate() {
    let mut bldr = DictBuilder::new_system();
    bldr.read_conn(MATRIX_10_10).unwrap();
    let lex = concat!(
        "index_form,left_id,right_id,cost,headword,pos1,pos2,pos3,pos4,pos5,pos6,reading_form,normalized_form,dictionary_form,split_a,split_b,split_c,word_structure,synonym_groups\n",
        "京都,6,6,5293,京都,名詞,固有名詞,地名,一般,*,*,キョウト,,,,,,,\n",
        "京都,6,6,5293,京都,名詞,固有名詞,地名,一般,*,*,キョウト,,\"京都,名詞,固有名詞,地名,一般,*,*,キョウト\",,,,,,\n"
    );
    assert_eq!(2, bldr.read_lexicon(lex.as_bytes()).unwrap());
    bldr.resolve().unwrap();

    assert_eq!(
        bldr.lexicon.resolved_entries()[1].dic_form,
        crate::dic::build::lexicon::ResolvedWordRef::Ref(DicWordRef::new(true, 4))
    );
}

#[test]
fn build_system_1word() {
    let mut bldr = DictBuilder::new_system();
    bldr.read_conn(MATRIX_10_10).unwrap();
    assert_eq!(
        1,
        bldr.read_lexicon(include_bytes!("test/data_1word.csv"))
            .unwrap()
    );
    bldr.resolve().unwrap();
    let mut built = Vec::new();
    bldr.compile(&mut built).unwrap();
    let dic = LoadedDictionary::load_system(&built).unwrap();

    let entry = dic.lexicon().lookup("京都".as_bytes(), 0).next().unwrap();
    assert_eq!(entry.word_id.dict().as_raw(), 0);
    let info = dic.lexicon().get_word_info(entry.word_id).unwrap();
    assert_eq!(info.headword(&dic), "京都");
    assert_eq!(info.reading_form(&dic), "キョウト");
}

#[test]
fn build_system_preserves_empty_and_explicit_equal_reading() {
    let mut bldr = DictBuilder::new_system();
    bldr.read_conn(MATRIX_10_10).unwrap();
    let lex = concat!(
        "index_form,left_id,right_id,cost,headword,pos1,pos2,pos3,pos4,pos5,pos6,reading_form,normalized_form,dictionary_form,split_a,split_b,split_c,word_structure,synonym_groups\n",
        "空読,6,6,5293,空読,名詞,普通名詞,一般,*,*,*,,,,,,,,\n",
        "同読,6,6,5293,同読,名詞,普通名詞,一般,*,*,*,同読,,,,,,,\n"
    );
    assert_eq!(2, bldr.read_lexicon(lex.as_bytes()).unwrap());
    bldr.resolve().unwrap();
    let mut built = Vec::new();
    bldr.compile(&mut built).unwrap();
    let dic = LoadedDictionary::load_system(&built).unwrap();

    let empty = dic.lexicon().lookup("空読".as_bytes(), 0).next().unwrap();
    let empty_info = dic.lexicon().get_word_info(empty.word_id).unwrap();
    assert_eq!(empty_info.reading_form(&dic), "");

    let explicit = dic.lexicon().lookup("同読".as_bytes(), 0).next().unwrap();
    let explicit_info = dic.lexicon().get_word_info(explicit.word_id).unwrap();
    assert_eq!(explicit_info.reading_form(&dic), "同読");
}

#[test]
fn build_system_sets_default_signature() {
    let mut bldr = DictBuilder::new_system();
    bldr.set_compile_time(UNIX_EPOCH + Duration::from_secs(1));
    bldr.set_description("abc");
    bldr.read_conn(MATRIX_10_10).unwrap();
    bldr.read_lexicon(include_bytes!("test/data_1word.csv"))
        .unwrap();
    bldr.resolve().unwrap();

    let mut built = Vec::new();
    bldr.compile(&mut built).unwrap();
    let dic = BinaryDictionary::load_system(&built).unwrap();

    assert_eq!(dic.description.reference(), "");
    assert_eq!(dic.description.signature().len(), 23);
    assert!(dic.description.signature()[..14]
        .chars()
        .all(|c| c.is_ascii_digit()));
    assert_eq!(&dic.description.signature()[14..15], "-");
    assert_eq!(&dic.description.signature()[15..], "00017862");
}

#[test]
fn compile_time_before_unix_epoch_fails_with_build_error() {
    let mut bldr = DictBuilder::new_system();
    bldr.set_compile_time(UNIX_EPOCH - Duration::from_secs(1));
    bldr.read_conn(MATRIX_10_10).unwrap();
    bldr.read_lexicon(include_bytes!("test/data_1word.csv"))
        .unwrap();
    bldr.resolve().unwrap();

    let mut built = Vec::new();
    claim::assert_matches!(
        bldr.compile(&mut built),
        Err(SudachiError::DictionaryCompilationError(DicBuildError {
            cause: BuildFailure::InvalidCompileTime,
            ..
        }))
    );
}

#[test]
fn build_system_3words() {
    let mut bldr = DictBuilder::new_system();
    bldr.read_conn(MATRIX_10_10).unwrap();
    assert_eq!(
        3,
        bldr.read_lexicon(include_bytes!("test/data_3words.csv"))
            .unwrap()
    );
    bldr.resolve().unwrap();
    let mut built = Vec::new();
    bldr.compile(&mut built).unwrap();
    let dic = LoadedDictionary::load_system(&built).unwrap();
    let mut iter = dic.lexicon().lookup("東京".as_bytes(), 0);
    let _short = iter.next().unwrap();
    let entry = iter.next().unwrap();
    assert_eq!(entry.end, 6);
    assert_eq!(entry.word_id.dict().as_raw(), 0);
    assert_eq!(iter.next(), None);
    let info = dic.lexicon().get_word_info(entry.word_id).unwrap();
    assert_eq!(info.headword(&dic), "京都");
}

#[test]
fn read_lexicon_after_resolve_fails() {
    let mut bldr = DictBuilder::new_system();
    bldr.read_conn(MATRIX_10_10).unwrap();
    assert_eq!(
        1,
        bldr.read_lexicon(include_bytes!("test/data_1word.csv"))
            .unwrap()
    );
    bldr.resolve().unwrap();

    claim::assert_matches!(
        bldr.read_lexicon(include_bytes!("test/data_2words_3w_refs.csv")),
        Err(SudachiError::DictionaryCompilationError(DicBuildError {
            cause: BuildFailure::InvalidBuilderState(_),
            ..
        }))
    );
}

#[test]
fn description_counts_exclude_phantom_entries() {
    let mut bldr = DictBuilder::new_system();
    bldr.read_conn(MATRIX_10_10).unwrap();
    let data = concat!(
        "index_form,left_id,right_id,cost,pos1,pos2,pos3,pos4,pos5,pos6,reading_form,normalized_form,dictionary_form,mode,split_a,split_b,word_structure\n",
        "舞台藝術,1,1,2816,名詞,普通名詞,一般,*,*,*,ブタイゲイジュツ,舞台芸術,,A,,,"
    );
    bldr.read_lexicon(data.as_bytes()).unwrap();
    bldr.resolve().unwrap();

    let mut built = Vec::new();
    bldr.compile(&mut built).unwrap();
    let dic = BinaryDictionary::load_system(&built).unwrap();
    let loaded = LoadedDictionary::load_system(&built).unwrap();

    assert_eq!(dic.description.num_total_entries(), 1);
    assert_eq!(dic.description.num_indexed_entries(), 1);
    assert_eq!(loaded.lexicon().size(), 1);
}

#[test]
fn build_user_dictionary_crossrefs() {
    let mut bldr = DictBuilder::new_system();
    bldr.read_conn(include_bytes!("test/matrix_10x10.def"))
        .unwrap();
    assert_eq!(
        3,
        bldr.read_lexicon(include_bytes!("test/data_3words.csv"))
            .unwrap()
    );
    bldr.resolve().unwrap();
    let mut system_bin = Vec::new();
    bldr.compile(&mut system_bin).unwrap();
    let dic = LoadedDictionary::load_system(&system_bin).unwrap();
    // user dictionary
    let mut bldr2 = DictBuilder::new_user(&dic);
    assert_eq!(
        2,
        bldr2
            .read_lexicon(include_bytes!("test/data_2words_3w_refs.csv"))
            .unwrap()
    );
    bldr2.resolve().unwrap();
    let mut user_dic = Vec::new();
    bldr2.compile(&mut user_dic).unwrap();
    let udic = BinaryDictionary::load_user(&user_dic).unwrap();
    let dic = dic.merge_dictionary(udic).unwrap();
    let mut iter = dic.lexicon_set.lookup("東".as_bytes(), 0);
    let entry_to = iter.next().unwrap();

    let mut iter = dic.lexicon_set.lookup("関東".as_bytes(), 0);
    let entry_kan = iter.next().unwrap();
    assert_eq!(entry_kan.word_id.dict().as_raw(), 1);
    let winfo = dic.lexicon_set.get_word_info(entry_kan.word_id).unwrap();
    assert_eq!(
        dic.lexicon_set.get_word_param(entry_kan.word_id),
        (4, 4, 4000)
    );
    assert_eq!(winfo.headword(&dic), "関");
    assert_eq!(winfo.a_unit_split().len(), 0);
    assert_eq!(winfo.synonym_group_ids(), [0, 1]);

    let entry_kanto = iter.next().unwrap();
    assert_eq!(entry_kanto.word_id.dict().as_raw(), 1);
    assert_eq!(
        dic.lexicon_set.get_word_param(entry_kanto.word_id),
        (5, 5, 5000)
    );
    let winfo = dic.lexicon_set.get_word_info(entry_kanto.word_id).unwrap();
    assert_eq!(winfo.headword(&dic), "関東");
    assert_eq!(winfo.a_unit_split(), [entry_kan.word_id, entry_to.word_id]);
    assert_eq!(winfo.b_unit_split(), [entry_kan.word_id, entry_to.word_id]);
    assert_eq!(iter.next(), None);
}

#[test]
fn build_user_sets_reference_to_system_signature() {
    let mut system = DictBuilder::new_system();
    system.set_compile_time(UNIX_EPOCH + Duration::from_secs(1));
    system.set_description("abc");
    system.read_conn(MATRIX_10_10).unwrap();
    system
        .read_lexicon(include_bytes!("test/data_1word.csv"))
        .unwrap();
    system.resolve().unwrap();

    let mut system_bin = Vec::new();
    system.compile(&mut system_bin).unwrap();
    let system_dic = LoadedDictionary::load_system(&system_bin).unwrap();
    let system_desc = BinaryDictionary::load_system(&system_bin)
        .unwrap()
        .description;

    let mut user = DictBuilder::new_user(&system_dic);
    user.read_lexicon(include_bytes!("test/data_1word.csv"))
        .unwrap();
    user.resolve().unwrap();

    let mut user_bin = Vec::new();
    user.compile(&mut user_bin).unwrap();
    let user_desc = BinaryDictionary::load_user(&user_bin).unwrap().description;

    assert_eq!(user_desc.signature(), "");
    assert_eq!(user_desc.reference(), system_desc.signature());
}

#[test]
fn fail_matrix_size_validation() {
    let mut bldr = DictBuilder::new_system();
    bldr.read_conn(MATRIX_10_10).unwrap();

    claim::assert_matches!(
        bldr.read_lexicon(
            concat!(
                "index_form,left_id,right_id,cost,headword,pos1,pos2,pos3,pos4,pos5,pos6,reading_form,normalized_form,dictionary_form,mode,split_a,split_b,word_structure,synonym_groups\n",
                "京都,10,5,5293,京都,名詞,固有名詞,地名,一般,*,*,キョウト,京都,,A,,,,"
            )
            .as_bytes(),
        ),
        Err(_)
    );

    let mut bldr = DictBuilder::new_system();
    bldr.read_conn(MATRIX_10_10).unwrap();
    claim::assert_matches!(
        bldr.read_lexicon(
            concat!(
                "index_form,left_id,right_id,cost,headword,pos1,pos2,pos3,pos4,pos5,pos6,reading_form,normalized_form,dictionary_form,mode,split_a,split_b,word_structure,synonym_groups\n",
                "京都,5,10,5293,京都,名詞,固有名詞,地名,一般,*,*,キョウト,京都,,A,,,,"
            )
            .as_bytes(),
        ),
        Err(_)
    );
}

#[test]
fn various_word_references_system() {
    let mut bldr = DictBuilder::new_system();
    bldr.read_conn(MATRIX_10_10).unwrap();
    assert_eq!(11, bldr.read_lexicon(WORDREF_SYSTEM).unwrap());
    bldr.resolve().unwrap();
    let mut data = Vec::new();
    bldr.compile(&mut data).unwrap();
    let dic = LoadedDictionary::load_system(&data).unwrap();
    assert_eq!(11, dic.lexicon().size());

    let tokyo = dic
        .lexicon()
        .lookup("トウキョウ".as_bytes(), 0)
        .next()
        .unwrap();
    let tokyo_info = dic.lexicon().get_word_info(tokyo.word_id).unwrap();
    assert_eq!("東京", tokyo_info.normalized_form(&dic));
    assert_eq!("東京", tokyo_info.dictionary_form(&dic));

    let east_tokyo = dic
        .lexicon()
        .lookup("東トウキョウ".as_bytes(), 0)
        .filter(|entry| entry.end == "東トウキョウ".len())
        .next()
        .unwrap();
    let east_tokyo_info = dic.lexicon().get_word_info(east_tokyo.word_id).unwrap();
    let structure = east_tokyo_info.c_unit_split();
    assert_eq!(2, structure.len());
    assert_eq!(
        "東",
        dic.lexicon()
            .get_word_info(structure[0])
            .unwrap()
            .headword(&dic)
    );
    assert_eq!(
        "東京B",
        dic.lexicon()
            .get_word_info(structure[1])
            .unwrap()
            .normalized_form(&dic)
    );
}

#[test]
fn various_word_references_user() {
    let mut bldr = DictBuilder::new_system();
    bldr.read_conn(MATRIX_10_10).unwrap();
    assert_eq!(11, bldr.read_lexicon(WORDREF_SYSTEM).unwrap());
    bldr.resolve().unwrap();
    let mut data = Vec::new();
    bldr.compile(&mut data).unwrap();
    let sys = LoadedDictionary::load_system(&data).unwrap();

    let mut user = DictBuilder::new_user(&sys);
    assert_eq!(5, user.read_lexicon(WORDREF_USER).unwrap());
    user.resolve().unwrap();
    let mut user_data = Vec::new();
    user.compile(&mut user_data).unwrap();

    let user_bin = BinaryDictionary::load_user(&user_data).unwrap();
    let merged = sys.merge_dictionary(user_bin).unwrap();
    let entries: Vec<_> = merged
        .lexicon_set
        .lookup("東京府".as_bytes(), 0)
        .filter(|entry| entry.end == "東京府".len())
        .collect();
    assert_eq!(3, entries.len());
    for entry in &entries {
        assert_eq!(entry.word_id.dict().as_raw(), 1);
    }

    let normalized: Vec<_> = entries
        .iter()
        .map(|entry| {
            let wi = merged.lexicon_set.get_word_info(entry.word_id).unwrap();
            let splits = wi.a_unit_split();
            assert_eq!(2, splits.len());
            merged
                .lexicon_set
                .get_word_info(splits[1])
                .unwrap()
                .normalized_form(&merged)
                .to_owned()
        })
        .collect();
    assert_eq!(vec!["府", "府2u", "府3"], normalized);
}

#[test]
fn resolve_user_entry_without_system_in_trie() {
    let mut bldr = DictBuilder::new_system();
    bldr.read_conn(MATRIX_10_10).unwrap();
    bldr.read_lexicon(include_bytes!("test/sys_no_entry.csv"))
        .unwrap();
    bldr.resolve().unwrap();
    let mut data = Vec::new();
    bldr.compile(&mut data).unwrap();
    let dic = LoadedDictionary::load_system(&data).unwrap();
    let mut iter = dic.lexicon().lookup("東京".as_bytes(), 0);
    let e = iter.next().unwrap();
    assert_eq!(e.end, 6);
    assert_eq!(iter.next(), None);
    drop(iter);

    let mut bldr = DictBuilder::new_user(&dic);
    bldr.read_lexicon(include_bytes!("test/data_2words_3w_refs.csv"))
        .unwrap();
    bldr.resolve().unwrap();
    let mut data2 = Vec::new();
    bldr.compile(&mut data2).unwrap();
    let udic = BinaryDictionary::load_user(&data2).unwrap();
    let dic = dic.merge_dictionary(udic).unwrap();
    let mut iter = dic.lexicon().lookup("関東".as_bytes(), 0);
    let _ = iter.next().unwrap();
    let e = iter.next().unwrap();
    assert_eq!(iter.next(), None);
    let winfo = dic.lexicon_set.get_word_info(e.word_id).unwrap();
    assert_eq!(winfo.a_unit_split().len(), 2);
}

#[test]
fn build_system_resolves_ambiguous_reference_by_reference_id() {
    let mut bldr = DictBuilder::new_system();
    bldr.read_conn(MATRIX_10_10).unwrap();
    let data = concat!(
        "index_form,left_id,right_id,cost,headword,pos1,pos2,pos3,pos4,pos5,pos6,reading_form,normalized_form,dictionary_form,mode,split_a,split_b,word_structure,synonym_groups,reference_id\n",
        "東京,1,1,100,東京,名詞,固有名詞,地名,一般,*,*,トウキョウ,,,A,,,,,tokyo-1\n",
        "東京,1,1,200,東京,名詞,固有名詞,地名,一般,*,*,トウキョウ,,,A,,,,,tokyo-2\n",
        "東都,1,1,300,東都,名詞,固有名詞,地名,一般,*,*,トウト,,\"東京,名詞,固有名詞,地名,一般,*,*,トウキョウ,tokyo-2\",A,,,,,\n"
    );
    bldr.read_lexicon(data.as_bytes()).unwrap();
    bldr.resolve().unwrap();

    let refs = bldr.lexicon.row_word_refs(false);
    assert_eq!(
        bldr.lexicon.resolved_entries()[2].dic_form,
        crate::dic::build::lexicon::ResolvedWordRef::Ref(refs[1])
    );
}

#[test]
fn duplicate_reference_id_fails_build() {
    let mut bldr = DictBuilder::new_system();
    bldr.read_conn(MATRIX_10_10).unwrap();
    let data = concat!(
        "index_form,left_id,right_id,cost,headword,pos1,pos2,pos3,pos4,pos5,pos6,reading_form,normalized_form,dictionary_form,mode,split_a,split_b,word_structure,synonym_groups,reference_id\n",
        "東京,1,1,100,東京,名詞,固有名詞,地名,一般,*,*,トウキョウ,,,A,,,,,dup-id\n",
        "京都,1,1,100,京都,名詞,固有名詞,地名,一般,*,*,キョウト,,,A,,,,,dup-id\n"
    );
    bldr.read_lexicon(data.as_bytes()).unwrap();

    claim::assert_matches!(
        bldr.resolve(),
        Err(SudachiError::DictionaryCompilationError(DicBuildError {
            cause: BuildFailure::InvalidSplit(_),
            ..
        }))
    );
}

#[test]
fn compiled_dictionary_preserves_reference_id_table() {
    let mut bldr = DictBuilder::new_system();
    bldr.read_conn(MATRIX_10_10).unwrap();
    let data = concat!(
        "index_form,left_id,right_id,cost,headword,pos1,pos2,pos3,pos4,pos5,pos6,reading_form,normalized_form,dictionary_form,mode,split_a,split_b,word_structure,synonym_groups,reference_id\n",
        "東京,1,1,100,東京,名詞,固有名詞,地名,一般,*,*,トウキョウ,,,A,,,,,tokyo-1\n"
    );
    bldr.read_lexicon(data.as_bytes()).unwrap();
    bldr.resolve().unwrap();

    let mut built = Vec::new();
    bldr.compile(&mut built).unwrap();
    let bin = BinaryDictionary::load_system(&built).unwrap();
    let loaded = LoadedDictionary::load_system(&built).unwrap();
    let wid = loaded
        .lexicon()
        .lookup("東京".as_bytes(), 0)
        .next()
        .unwrap()
        .word_id;
    let reference_ids = bin.reference_id_table().unwrap();
    assert_eq!(
        reference_ids.get(&wid.entry().as_raw()).map(|s| s.as_str()),
        Some("tokyo-1")
    );
}

#[test]
fn user_reference_id_prefers_user_entries() {
    let mut system = DictBuilder::new_system();
    system.read_conn(MATRIX_10_10).unwrap();
    system
        .read_lexicon(
            concat!(
                "index_form,left_id,right_id,cost,headword,pos1,pos2,pos3,pos4,pos5,pos6,reading_form,normalized_form,dictionary_form,mode,split_a,split_b,word_structure,synonym_groups,reference_id\n",
                "東京,1,1,100,東京,名詞,固有名詞,地名,一般,*,*,トウキョウ,,,A,,,,,tokyo-sys\n"
            )
            .as_bytes(),
        )
        .unwrap();
    system.resolve().unwrap();
    let mut system_bin = Vec::new();
    system.compile(&mut system_bin).unwrap();
    let system_dic = LoadedDictionary::load_system(&system_bin).unwrap();

    let mut user = DictBuilder::new_user(&system_dic);
    user.read_lexicon(
        concat!(
            "index_form,left_id,right_id,cost,headword,pos1,pos2,pos3,pos4,pos5,pos6,reading_form,normalized_form,dictionary_form,mode,split_a,split_b,word_structure,synonym_groups,reference_id\n",
            "東京,1,1,110,東京,名詞,固有名詞,地名,一般,*,*,トウキョウ,,,A,,,,,tokyo-user\n",
            "東都,1,1,120,東都,名詞,固有名詞,地名,一般,*,*,トウト,,,B,\"東京,名詞,固有名詞,地名,一般,*,*,トウキョウ,tokyo-user\",,,,\n"
        )
        .as_bytes(),
    )
    .unwrap();
    user.resolve().unwrap();

    let refs = user.lexicon.row_word_refs(true);
    assert_eq!(user.lexicon.resolved_entries()[1].splits_a, [refs[0]]);
}

#[test]
fn user_reference_id_falls_back_to_system_entries() {
    let mut system = DictBuilder::new_system();
    system.read_conn(MATRIX_10_10).unwrap();
    system
        .read_lexicon(
            concat!(
                "index_form,left_id,right_id,cost,headword,pos1,pos2,pos3,pos4,pos5,pos6,reading_form,normalized_form,dictionary_form,mode,split_a,split_b,word_structure,synonym_groups,reference_id\n",
                "東京,1,1,100,東京,名詞,固有名詞,地名,一般,*,*,トウキョウ,,,A,,,,,tokyo-sys\n"
            )
            .as_bytes(),
        )
        .unwrap();
    system.resolve().unwrap();
    let mut system_bin = Vec::new();
    system.compile(&mut system_bin).unwrap();
    let system_dic = LoadedDictionary::load_system(&system_bin).unwrap();

    let mut user = DictBuilder::new_user(&system_dic);
    user.read_lexicon(
        concat!(
            "index_form,left_id,right_id,cost,headword,pos1,pos2,pos3,pos4,pos5,pos6,reading_form,normalized_form,dictionary_form,mode,split_a,split_b,word_structure,synonym_groups\n",
            "東都,1,1,120,東都,名詞,固有名詞,地名,一般,*,*,トウト,,,B,\"東京,名詞,固有名詞,地名,一般,*,*,トウキョウ,tokyo-sys\",,,,\n"
        )
        .as_bytes(),
    )
    .unwrap();
    user.resolve().unwrap();

    let sys_ref = DicWordRef::new(
        true,
        system_dic.lexicon().system_word_ids_in_order()[0]
            .entry()
            .as_raw(),
    );
    assert_eq!(user.lexicon.resolved_entries()[0].splits_a, [sys_ref]);
}

#[test]
fn user_reference_id_triple_mismatch_fails() {
    let mut system = DictBuilder::new_system();
    system.read_conn(MATRIX_10_10).unwrap();
    system
        .read_lexicon(
            concat!(
                "index_form,left_id,right_id,cost,headword,pos1,pos2,pos3,pos4,pos5,pos6,reading_form,normalized_form,dictionary_form,mode,split_a,split_b,word_structure,synonym_groups,reference_id\n",
                "東京,1,1,100,東京,名詞,固有名詞,地名,一般,*,*,トウキョウ,,,A,,,,,tokyo-sys\n"
            )
            .as_bytes(),
        )
        .unwrap();
    system.resolve().unwrap();
    let mut system_bin = Vec::new();
    system.compile(&mut system_bin).unwrap();
    let system_dic = LoadedDictionary::load_system(&system_bin).unwrap();

    let mut user = DictBuilder::new_user(&system_dic);
    user.read_lexicon(
        concat!(
            "index_form,left_id,right_id,cost,headword,pos1,pos2,pos3,pos4,pos5,pos6,reading_form,normalized_form,dictionary_form,mode,split_a,split_b,word_structure,synonym_groups\n",
            "東都,1,1,120,東都,名詞,固有名詞,地名,一般,*,*,トウト,,,B,\"大阪,名詞,固有名詞,地名,一般,*,*,オオサカ,tokyo-sys\",,,,\n"
        )
        .as_bytes(),
    )
    .unwrap();

    claim::assert_matches!(
        user.resolve(),
        Err(SudachiError::DictionaryCompilationError(DicBuildError {
            cause: BuildFailure::InvalidSplitWordReference(_),
            ..
        }))
    );
}

#[test]
fn user_reference_without_reference_id_prefers_first_user_entry() {
    let mut system = DictBuilder::new_system();
    system.read_conn(MATRIX_10_10).unwrap();
    system
        .read_lexicon(include_bytes!("test/data_1word.csv"))
        .unwrap();
    system.resolve().unwrap();
    let mut system_bin = Vec::new();
    system.compile(&mut system_bin).unwrap();
    let system_dic = LoadedDictionary::load_system(&system_bin).unwrap();

    let mut user = DictBuilder::new_user(&system_dic);
    user.read_lexicon(
        concat!(
            "index_form,left_id,right_id,cost,headword,pos1,pos2,pos3,pos4,pos5,pos6,reading_form,normalized_form,dictionary_form,mode,split_a,split_b,word_structure,synonym_groups\n",
            "東京,1,1,110,東京,名詞,固有名詞,地名,一般,*,*,トウキョウ,,,A,,,,\n",
            "東京,1,1,111,東京,名詞,固有名詞,地名,一般,*,*,トウキョウ,,,A,,,,\n",
            "東都,1,1,120,東都,名詞,固有名詞,地名,一般,*,*,トウト,,,B,\"東京,名詞,固有名詞,地名,一般,*,*,トウキョウ\",,,,\n"
        )
        .as_bytes(),
    )
    .unwrap();
    user.resolve().unwrap();

    let refs = user.lexicon.row_word_refs(true);
    assert_eq!(user.lexicon.resolved_entries()[2].splits_a, [refs[0]]);
}
