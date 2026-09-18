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

#[test]
fn split_user_ref_in_v0_format() {
    let mut sys = DictBuilder::new_system();
    sys.read_conn(MATRIX_10_10).unwrap();
    sys.read_lexicon(
        concat!("東京,1,1,2816,東京,名詞,普通名詞,一般,*,*,*,トウキョウ,東京,*,A,*,*,*,*\n")
            .as_bytes(),
    )
    .unwrap();
    sys.resolve().unwrap();
    let mut sys_data = Vec::new();
    sys.compile(&mut sys_data).unwrap();
    let sys_dic = LoadedDictionary::load_system(&sys_data).unwrap();

    let mut user = DictBuilder::new_user(&sys_dic);
    user.read_lexicon(
        concat!(
            "東京都,2,2,5320,東京都,名詞,固有名詞,地名,一般,*,*,トウキョウト,東京都,*,B,0/U1,0/U1,0/U1,*\n",
            "都,2,2,2914,都,名詞,普通名詞,一般,*,*,*,ト,都,*,A,*,*,*,*\n"
        )
        .as_bytes(),
    )
    .unwrap();
    user.resolve().unwrap();
    let mut user_data = Vec::new();
    user.compile(&mut user_data).unwrap();
    let merged = sys_dic
        .merge_dictionary(BinaryDictionary::load_user(&user_data).unwrap())
        .unwrap();

    let tokyo = merged
        .lexicon_set
        .lookup("東京".as_bytes(), 0)
        .find(|e| e.word_id.dict().as_raw() == 0)
        .unwrap()
        .word_id;
    let to = merged
        .lexicon_set
        .lookup("都".as_bytes(), 0)
        .find(|e| e.word_id.dict().as_raw() == 1)
        .unwrap()
        .word_id;
    let tokyoto = merged
        .lexicon_set
        .lookup("東京都".as_bytes(), 0)
        .find(|e| e.word_id.dict().as_raw() == 1)
        .unwrap()
        .word_id;
    let wi = merged.lexicon_set.get_word_info(tokyoto).unwrap();
    assert_eq!(wi.a_unit_split(), [tokyo, to]);
    assert_eq!(wi.b_unit_split(), [tokyo, to]);
    assert_eq!(wi.word_structure(), [tokyo, to]);
}

#[test]
fn split_user_entrykey_ref_system_in_v0_format() {
    let mut sys = DictBuilder::new_system();
    sys.read_conn(MATRIX_10_10).unwrap();
    sys.read_lexicon(
        concat!("東京,1,1,2816,東京,名詞,普通名詞,一般,*,*,*,トウキョウ,東京,*,A,*,*,*,*\n")
            .as_bytes(),
    )
    .unwrap();
    sys.resolve().unwrap();
    let mut sys_data = Vec::new();
    sys.compile(&mut sys_data).unwrap();
    let sys_dic = LoadedDictionary::load_system(&sys_data).unwrap();

    let mut user = DictBuilder::new_user(&sys_dic);
    user.read_lexicon(
        concat!(
            "東京都,2,2,5320,東京都,名詞,固有名詞,地名,一般,*,*,トウキョウト,東京都,*,B,\"東京,名詞,普通名詞,一般,*,*,*,トウキョウ/U1\",*,*,*\n",
            "都,2,2,2914,都,名詞,普通名詞,一般,*,*,*,ト,都,*,A,*,*,*,*\n"
        )
        .as_bytes(),
    )
    .unwrap();
    user.resolve().unwrap();
    let mut user_data = Vec::new();
    user.compile(&mut user_data).unwrap();
    let merged = sys_dic
        .merge_dictionary(BinaryDictionary::load_user(&user_data).unwrap())
        .unwrap();

    let tokyo = merged
        .lexicon_set
        .lookup("東京".as_bytes(), 0)
        .find(|e| e.word_id.dict().as_raw() == 0)
        .unwrap()
        .word_id;
    let to = merged
        .lexicon_set
        .lookup("都".as_bytes(), 0)
        .find(|e| e.word_id.dict().as_raw() == 1)
        .unwrap()
        .word_id;
    let tokyoto = merged
        .lexicon_set
        .lookup("東京都".as_bytes(), 0)
        .find(|e| e.word_id.dict().as_raw() == 1)
        .unwrap()
        .word_id;
    let wi = merged.lexicon_set.get_word_info(tokyoto).unwrap();
    assert_eq!(wi.a_unit_split(), [tokyo, to]);
}

#[test]
fn split_user_entrykey_ref_user_in_v0_format() {
    let mut sys = DictBuilder::new_system();
    sys.read_conn(MATRIX_10_10).unwrap();
    sys.read_lexicon(
        concat!("東京,1,1,2816,東京,名詞,普通名詞,一般,*,*,*,トウキョウ,東京,*,A,*,*,*,*\n")
            .as_bytes(),
    )
    .unwrap();
    sys.resolve().unwrap();
    let mut sys_data = Vec::new();
    sys.compile(&mut sys_data).unwrap();
    let sys_dic = LoadedDictionary::load_system(&sys_data).unwrap();

    let mut user = DictBuilder::new_user(&sys_dic);
    user.read_lexicon(
        concat!(
            "東京都,2,2,5320,東京都,名詞,固有名詞,地名,一般,*,*,トウキョウト,東京都,*,B,\"0/都,名詞,普通名詞,一般,*,*,*,ト\",*,*,*\n",
            "都,2,2,2914,都,名詞,普通名詞,一般,*,*,*,ト,都,*,A,*,*,*,*\n"
        )
        .as_bytes(),
    )
    .unwrap();
    user.resolve().unwrap();
    let mut user_data = Vec::new();
    user.compile(&mut user_data).unwrap();
    let merged = sys_dic
        .merge_dictionary(BinaryDictionary::load_user(&user_data).unwrap())
        .unwrap();

    let tokyo = merged
        .lexicon_set
        .lookup("東京".as_bytes(), 0)
        .find(|e| e.word_id.dict().as_raw() == 0)
        .unwrap()
        .word_id;
    let to = merged
        .lexicon_set
        .lookup("都".as_bytes(), 0)
        .find(|e| e.word_id.dict().as_raw() == 1)
        .unwrap()
        .word_id;
    let tokyoto = merged
        .lexicon_set
        .lookup("東京都".as_bytes(), 0)
        .find(|e| e.word_id.dict().as_raw() == 1)
        .unwrap()
        .word_id;
    let wi = merged.lexicon_set.get_word_info(tokyoto).unwrap();
    assert_eq!(wi.a_unit_split(), [tokyo, to]);
}

#[test]
#[ignore = "sudachi.rs currently allows system dictionary_form reference from user lexicon"]
fn fail_dictionary_form_in_system_in_v0_format() {
    let mut sys = DictBuilder::new_system();
    sys.read_conn(MATRIX_10_10).unwrap();
    sys.read_lexicon(
        "行く,1,1,5105,行く,動詞,非自立可能,*,*,五段-カ行,終止形-一般,イク,行く,*,A,*,*,*,*"
            .as_bytes(),
    )
    .unwrap();
    sys.resolve().unwrap();
    let mut sys_data = Vec::new();
    sys.compile(&mut sys_data).unwrap();
    let sys_dic = LoadedDictionary::load_system(&sys_data).unwrap();

    let mut user = DictBuilder::new_user(&sys_dic);
    user.read_lexicon(
        "行っ,2,2,5122,行っ,動詞,非自立可能,*,*,五段-カ行,連用形-促音便,イッ,行く,\"行く,動詞,非自立可能,*,*,五段-カ行,終止形-一般,イク\",A,*,*,*,*".as_bytes(),
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
fn word_id_too_big_dicform_in_v0_format() {
    let mut bldr = DictBuilder::new_system();
    bldr.read_conn(MATRIX_10_10).unwrap();
    bldr.read_lexicon(
        "京都,5,5,5293,京都,名詞,固有名詞,地名,一般,*,*,キョウト,京都,5,A,*,*,*,*".as_bytes(),
    )
    .unwrap();
    claim::assert_matches!(
        bldr.resolve(),
        Err(SudachiError::DictionaryCompilationError(DicBuildError {
            cause: BuildFailure::InvalidSplitWordReference(_),
            ..
        }))
    );
}

#[test]
fn word_id_too_big_split_a_in_v0_format() {
    let mut bldr = DictBuilder::new_system();
    bldr.read_conn(MATRIX_10_10).unwrap();
    bldr.read_lexicon(
        "京都,5,5,5293,京都,名詞,固有名詞,地名,一般,*,*,キョウト,京都,*,C,0/5,*,*,*".as_bytes(),
    )
    .unwrap();
    claim::assert_matches!(
        bldr.resolve(),
        Err(SudachiError::DictionaryCompilationError(DicBuildError {
            cause: BuildFailure::InvalidSplitWordReference(_),
            ..
        }))
    );
}

#[test]
fn word_id_too_big_split_b_in_v0_format() {
    let mut bldr = DictBuilder::new_system();
    bldr.read_conn(MATRIX_10_10).unwrap();
    bldr.read_lexicon(
        "京都,5,5,5293,京都,名詞,固有名詞,地名,一般,*,*,キョウト,京都,*,C,*,0/5,*,*".as_bytes(),
    )
    .unwrap();
    claim::assert_matches!(
        bldr.resolve(),
        Err(SudachiError::DictionaryCompilationError(DicBuildError {
            cause: BuildFailure::InvalidSplitWordReference(_),
            ..
        }))
    );
}

#[test]
fn word_id_too_big_word_structure_in_v0_format() {
    let mut bldr = DictBuilder::new_system();
    bldr.read_conn(MATRIX_10_10).unwrap();
    bldr.read_lexicon(
        "京都,5,5,5293,京都,名詞,固有名詞,地名,一般,*,*,キョウト,京都,*,C,*,*,0/5,*".as_bytes(),
    )
    .unwrap();
    claim::assert_matches!(
        bldr.resolve(),
        Err(SudachiError::DictionaryCompilationError(DicBuildError {
            cause: BuildFailure::InvalidSplitWordReference(_),
            ..
        }))
    );
}

#[test]
fn word_id_too_big_dicform_userdic_insystem_in_v0_format() {
    let mut bldr = DictBuilder::new_system();
    bldr.read_conn(MATRIX_10_10).unwrap();
    bldr.read_lexicon(
        "京都,5,5,5293,京都,名詞,固有名詞,地名,一般,*,*,キョウト,京都,*,A,*,*,*,*".as_bytes(),
    )
    .unwrap();
    bldr.resolve().unwrap();
    let mut data = Vec::new();
    bldr.compile(&mut data).unwrap();
    let dic = LoadedDictionary::load_system(&data).unwrap();
    let mut bldr = DictBuilder::new_user(&dic);
    bldr.read_lexicon("東,6,6,5293,東,名詞,一般,*,*,*,*,ヒガシ,*,10,A,*,*,*,*".as_bytes())
        .unwrap();
    claim::assert_matches!(
        bldr.resolve(),
        Err(SudachiError::DictionaryCompilationError(DicBuildError {
            cause: BuildFailure::InvalidSplitWordReference(_),
            ..
        }))
    );
}

#[test]
fn word_id_too_big_dicform_userdic_inuser_in_v0_format() {
    let mut bldr = DictBuilder::new_system();
    bldr.read_conn(MATRIX_10_10).unwrap();
    bldr.read_lexicon(
        "京都,5,5,5293,京都,名詞,固有名詞,地名,一般,*,*,キョウト,京都,*,A,*,*,*,*".as_bytes(),
    )
    .unwrap();
    bldr.resolve().unwrap();
    let mut data = Vec::new();
    bldr.compile(&mut data).unwrap();
    let dic = LoadedDictionary::load_system(&data).unwrap();
    let mut bldr = DictBuilder::new_user(&dic);
    bldr.read_lexicon("東,6,6,5293,東,名詞,一般,*,*,*,*,ヒガシ,*,U15,A,*,*,*,*".as_bytes())
        .unwrap();
    claim::assert_matches!(
        bldr.resolve(),
        Err(SudachiError::DictionaryCompilationError(DicBuildError {
            cause: BuildFailure::InvalidSplitWordReference(_),
            ..
        }))
    );
}
