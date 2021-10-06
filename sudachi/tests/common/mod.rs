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

use std::path::{Path, PathBuf};

extern crate sudachi;
use self::sudachi::dic::dictionary::JapaneseDictionary;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::sync::Arc;
use sudachi::analysis::stateless_tokenizer::StatelessTokenizer;
use sudachi::config::Config;
use sudachi::dic::{grammar::Grammar, header::Header, lexicon::Lexicon};
use sudachi::prelude::*;

pub fn dictionary_bytes_from_path<P: AsRef<Path>>(dictionary_path: P) -> SudachiResult<Vec<u8>> {
    let dictionary_path = dictionary_path.as_ref();
    let dictionary_stat = fs::metadata(&dictionary_path)?;
    let mut dictionary_file = File::open(dictionary_path)?;
    let mut dictionary_bytes = Vec::with_capacity(dictionary_stat.len() as usize);
    dictionary_file.read_to_end(&mut dictionary_bytes)?;

    Ok(dictionary_bytes)
}

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
}

pub struct TestTokenizer {
    tok: StatelessTokenizer<Arc<JapaneseDictionary>>,
}

#[allow(unused)]
impl TestTokenizer {
    pub fn new() -> TestTokenizer {
        let dict = JapaneseDictionary::from_cfg(&TEST_CONFIG).expect("failed to make dictionary");
        let tok = StatelessTokenizer::new(Arc::new(dict));
        return TestTokenizer { tok };
    }

    pub fn tokenize<'a>(
        &'a self,
        data: &'a str,
        mode: Mode,
    ) -> MorphemeList<&'a JapaneseDictionary> {
        let result = self.tok.tokenize(data, mode, false);
        result.expect("tokenization failed")
    }

    pub fn dict(&self) -> &JapaneseDictionary {
        &self.tok.as_dict()
    }
}

// lazy_static! {
//     pub static ref TEST_TOKENIZER: TestTokenizer = TestTokenizer::new();
// }
