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

use std::path::PathBuf;

extern crate sudachi;
use sudachi::config::Config;
use sudachi::dic::{grammar::Grammar, header::Header, lexicon::Lexicon};
use sudachi::prelude::*;

lazy_static! {
    pub static ref TEST_CONFIG: Config = {
        let test_config_path = "tests/resources/sudachi.json";
        let conf = Config::new(Some(PathBuf::from(test_config_path)), None, None)
            .expect("Failed to read config file for test");
        println!("{:?}", conf);
        conf
    };
    static ref DICTIONARY_BYTES: Vec<u8> = {
        let dictionary_path = TEST_CONFIG
            .system_dict
            .clone()
            .expect("No system dictionary set in config");
        let dictionary_bytes = dictionary_bytes_from_path(dictionary_path)
            .expect("Failed to read dictionary from path");
        dictionary_bytes
    };
    static ref USER_DICTIONARY_BYTES: Vec<Box<[u8]>> = {
        let mut bytes = Vec::with_capacity(TEST_CONFIG.user_dicts.len());
        for pb in &TEST_CONFIG.user_dicts {
            let storage_buf = dictionary_bytes_from_path(pb)
                .expect("Failed to get user dictionary bytes from file");
            bytes.push(storage_buf.into_boxed_slice());
        }
        bytes
    };
    pub static ref HEADER: Header =
        Header::new(&DICTIONARY_BYTES).expect("Failed to create Header for tests");
    pub static ref GRAMMAR: Grammar<'static> =
        Grammar::new(&DICTIONARY_BYTES, Header::STORAGE_SIZE)
            .expect("Failed to read grammar for tests");
    pub static ref LEXICON: Lexicon<'static> = {
        let offset = Header::STORAGE_SIZE + GRAMMAR.storage_size;
        Lexicon::new(&DICTIONARY_BYTES, offset, HEADER.has_synonym_group_ids())
            .expect("Failed to read lexicon for tests")
    };
    pub static ref TOKENIZER: Tokenizer<'static> =
        Tokenizer::from_dictionary_bytes(&DICTIONARY_BYTES, &USER_DICTIONARY_BYTES, &TEST_CONFIG,)
            .expect("Failed to create Tokenizer for tests");
}
