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

use std::time::{Duration, UNIX_EPOCH};

mod common;
use common::{TestTokenizer, GRAMMAR};
use sudachi::dic::build::DictBuilder;
use sudachi::dic::dictionary::JapaneseDictionary;
use sudachi::dic::error::DictionaryCompatibilityError;
use sudachi::dic::storage::{Storage, SudachiDicData};
use sudachi::error::SudachiError;

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

// fn creat_with_merging_settings
// fn creat_with_merging_null_settings
