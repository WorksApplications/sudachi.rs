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

extern crate sudachi;

use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

use sudachi::analysis::stateful_tokenizer::StatefulTokenizer;
use sudachi::analysis::stateless_tokenizer::StatelessTokenizer;
use sudachi::config::{Config, ConfigBuilder};
use sudachi::dic::dictionary::JapaneseDictionary;
use sudachi::dic::{grammar::Grammar, header::Header, lexicon::Lexicon, DictionaryLoader};
use sudachi::prelude::*;

use lazy_static::lazy_static;
use sudachi::analysis::Tokenize;
use sudachi::dic::build::DictBuilder;
use sudachi::dic::storage::{Storage, SudachiDicData};

pub fn dictionary_bytes_from_path<P: AsRef<Path>>(dictionary_path: P) -> SudachiResult<Vec<u8>> {
    let dictionary_path = dictionary_path.as_ref();
    let dictionary_stat = fs::metadata(&dictionary_path)?;
    let mut dictionary_file = File::open(dictionary_path)?;
    let mut dictionary_bytes = Vec::with_capacity(dictionary_stat.len() as usize);
    dictionary_file.read_to_end(&mut dictionary_bytes)?;

    Ok(dictionary_bytes)
}

pub const LEX_CSV: &[u8] = include_bytes!("../resources/lex.csv");

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
        Header::parse(&DICTIONARY_BYTES).expect("Failed to create Header for tests");
    pub static ref GRAMMAR: Grammar<'static> =
        Grammar::parse(&DICTIONARY_BYTES, Header::STORAGE_SIZE)
            .expect("Failed to read grammar for tests");
    pub static ref LEXICON: Lexicon<'static> = {
        let offset = Header::STORAGE_SIZE + GRAMMAR.storage_size;
        let mut lex = Lexicon::parse(&DICTIONARY_BYTES, offset, HEADER.has_synonym_group_ids())
            .expect("Failed to read lexicon for tests");
        lex.set_dic_id(0);
        lex
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
    ) -> MorphemeList<Arc<JapaneseDictionary>> {
        let result = self.tok.tokenize(data, mode, false);
        result.expect("tokenization failed")
    }

    pub fn dict(&self) -> &JapaneseDictionary {
        &self.tok.as_dict()
    }
}

pub struct TestTokenizerBuilder<'a> {
    pub conn: Option<&'a [u8]>,
    pub system: &'a [u8],
    pub user: Vec<&'a [u8]>,
    pub mode: Mode,
    pub debug: bool,
    pub config: Option<&'a [u8]>,
}

#[allow(unused)]
impl<'a> TestTokenizerBuilder<'a> {
    pub fn user(mut self, data: &'a [u8]) -> Self {
        self.user.push(data);
        self
    }

    pub fn mode(mut self, mode: Mode) -> Self {
        self.mode = mode;
        self
    }

    pub fn debug(mut self, debug: bool) -> Self {
        self.debug = debug;
        self
    }

    pub fn config(mut self, data: &'static [u8]) -> Self {
        self.config = Some(data);
        self
    }

    pub fn build(self) -> TestStatefulTokenizer {
        let mut sys = DictBuilder::new_system();
        sys.read_conn(
            self.conn
                .unwrap_or(include_bytes!("../resources/matrix_10x10.def")),
        )
        .unwrap();
        sys.read_lexicon(self.system).unwrap();
        sys.resolve().unwrap();
        let mut sys_bytes = Vec::new();
        sys.compile(&mut sys_bytes).unwrap();

        let mut data = SudachiDicData::new(Storage::Owned(sys_bytes));

        if !self.user.is_empty() {
            let dic =
                DictionaryLoader::read_system_dictionary(unsafe { data.system_static_slice() })
                    .unwrap()
                    .to_loaded()
                    .unwrap();

            for u in self.user {
                let mut ubld = DictBuilder::new_user(&dic);
                ubld.read_lexicon(u).unwrap();
                ubld.resolve().unwrap();
                let mut user_bytes = Vec::new();
                ubld.compile(&mut user_bytes).unwrap();
                data.add_user(Storage::Owned(user_bytes));
            }
        }

        let config = match self.config {
            None => TEST_CONFIG.clone(),
            Some(data) => ConfigBuilder::from_bytes(data).unwrap().build(),
        };

        let dic = JapaneseDictionary::from_cfg_storage(&config, data).unwrap();
        let rcdic = Rc::new(dic);

        TestStatefulTokenizer {
            tok: StatefulTokenizer::create(rcdic.clone(), self.debug, self.mode),
            result: MorphemeList::empty(rcdic),
        }
    }
}

pub struct TestStatefulTokenizer {
    pub tok: StatefulTokenizer<Rc<JapaneseDictionary>>,
    pub result: MorphemeList<Rc<JapaneseDictionary>>,
}

#[allow(unused)]
impl TestStatefulTokenizer {
    pub fn new_built(mode: Mode) -> TestStatefulTokenizer {
        let dic = Rc::new(JapaneseDictionary::from_cfg(&TEST_CONFIG).expect("works"));
        Self {
            tok: StatefulTokenizer::new(dic.clone(), mode),
            result: MorphemeList::empty(dic),
        }
    }

    pub fn builder(system: &[u8]) -> TestTokenizerBuilder {
        TestTokenizerBuilder {
            system,
            user: Vec::new(),
            conn: None,
            mode: Mode::C,
            debug: false,
            config: None,
        }
    }

    pub fn tokenize(&mut self, data: &str) -> &MorphemeList<Rc<JapaneseDictionary>> {
        self.tok.reset().push_str(data);
        self.tok.do_tokenize().expect("tokenization failed");
        self.result
            .collect_results(&mut self.tok)
            .expect("collection failed");
        &self.result
    }

    pub fn dict(&self) -> &JapaneseDictionary {
        self.tok.dict()
    }

    pub fn set_mode(&mut self, mode: Mode) -> Mode {
        self.tok.set_mode(mode)
    }
}
