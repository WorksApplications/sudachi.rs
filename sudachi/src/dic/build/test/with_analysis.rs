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

use crate::analysis::stateful_tokenizer::StatefulTokenizer;
use crate::analysis::stateless_tokenizer::StatelessTokenizer;
use crate::analysis::{Mode, Tokenize};
use crate::config::Config;
use crate::dic::build::DictBuilder;
use crate::dic::dictionary::JapaneseDictionary;
use crate::dic::subset::InfoSubset;
use crate::prelude::MorphemeList;
use std::fmt::{Debug, Write as FmtWrite};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use tempfile::NamedTempFile;

static SYSTEM_LEX: &[u8] = include_bytes!("lex.csv");
static USER1_LEX: &[u8] = include_bytes!("user1.csv");
static USER2_LEX: &[u8] = include_bytes!("user2.csv");

#[derive(Default)]
struct ConfigTestSupport {
    tmpfiles: Vec<NamedTempFile>,
    user: Vec<PathBuf>,
    system: Option<PathBuf>,
}

static CONFIG_TEMPLATE: &str = include_str!("test_config_template.json");

impl ConfigTestSupport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_user(&mut self) -> BufWriter<&mut File> {
        let tfile = tempfile::Builder::new()
            .prefix("sudachi_ud")
            .suffix(".dic")
            .tempfile()
            .unwrap();
        self.user.push(tfile.path().to_path_buf());
        self.tmpfiles.push(tfile);
        let file = self.tmpfiles.last_mut().unwrap().as_file_mut();
        BufWriter::new(file)
    }

    pub fn make_system(&mut self) -> BufWriter<&mut File> {
        let tfile = tempfile::Builder::new()
            .prefix("sudachi_sd")
            .suffix(".dic")
            .tempfile()
            .unwrap();
        self.system = Some(tfile.path().to_path_buf());
        self.tmpfiles.push(tfile);
        let file = self.tmpfiles.last_mut().unwrap().as_file_mut();
        BufWriter::new(file)
    }

    fn concat<D: Debug, I: Iterator<Item = D>>(mut data: I, sep: &str) -> String {
        let mut prev = match data.next() {
            None => return String::new(),
            Some(x) => x,
        };

        let mut result = String::new();

        loop {
            match data.next() {
                Some(x) => {
                    write!(result, "{:?}{}", prev, sep).unwrap();
                    prev = x;
                }
                None => break,
            }
        }
        write!(result, "{:?}", prev).unwrap();
        result
    }

    pub fn config(&self) -> Config {
        let cfg = String::from(CONFIG_TEMPLATE);
        let cfg = cfg.replace(
            "\"$system_dict\"",
            &format!("{:?}", self.system.as_ref().unwrap()),
        );

        let user_dicts = Self::concat(self.user.iter(), ", ");
        let cfg = cfg.replace("\"$user_dict\"", &format!("[{}]", user_dicts));

        let mut tfile = tempfile::Builder::new()
            .prefix("sudachi_cfg")
            .suffix(".json")
            .tempfile()
            .unwrap();
        tfile.write_all(cfg.as_bytes()).unwrap();

        Config::new(Some(tfile.path().to_path_buf()), None, None).unwrap()
    }
}

#[test]
fn system_only_1() {
    let mut cfgb = ConfigTestSupport::new();
    let mut dic = DictBuilder::new_system();
    dic.read_conn(super::MATRIX_10_10).unwrap();
    dic.read_lexicon(SYSTEM_LEX).unwrap();
    dic.resolve().unwrap();
    dic.compile(&mut cfgb.make_system()).unwrap();

    let cfg = cfgb.config();
    let jd = JapaneseDictionary::from_cfg(&cfg).unwrap();
    let tok = StatelessTokenizer::new(&jd);
    let result = tok.tokenize("東京にいく", Mode::C, false).unwrap();
    assert_eq!(result.len(), 3);
}

#[test]
fn system_plus_user_1() {
    let mut cfgb = ConfigTestSupport::new();
    let mut dic = DictBuilder::new_system();
    dic.read_conn(super::MATRIX_10_10).unwrap();
    dic.read_lexicon(SYSTEM_LEX).unwrap();
    dic.resolve().unwrap();
    dic.compile(&mut cfgb.make_system()).unwrap();

    let jd = JapaneseDictionary::from_cfg(&cfgb.config()).unwrap();
    let mut dic2 = DictBuilder::new_user(&jd);
    dic2.read_lexicon(USER1_LEX).unwrap();
    dic2.resolve().unwrap();
    dic2.compile(&mut cfgb.add_user()).unwrap();
    let jd2 = JapaneseDictionary::from_cfg(&cfgb.config()).unwrap();
    let tok = StatelessTokenizer::new(&jd2);
    let result = tok.tokenize("すだちにいく", Mode::C, false).unwrap();
    assert_eq!(result.len(), 3);
    assert_eq!(result.get(0).word_id().dic(), 1);
}

#[test]
fn system_plus_user_2() {
    let mut cfgb = ConfigTestSupport::new();
    let mut dic = DictBuilder::new_system();
    dic.read_conn(super::MATRIX_10_10).unwrap();
    dic.read_lexicon(SYSTEM_LEX).unwrap();
    dic.resolve().unwrap();
    dic.compile(&mut cfgb.make_system()).unwrap();

    let jd = JapaneseDictionary::from_cfg(&cfgb.config()).unwrap();
    let mut dic2 = DictBuilder::new_user(&jd);
    dic2.read_lexicon(USER1_LEX).unwrap();
    dic2.resolve().unwrap();
    dic2.compile(&mut cfgb.add_user()).unwrap();
    let mut dic2 = DictBuilder::new_user(&jd);
    dic2.read_lexicon(USER2_LEX).unwrap();
    dic2.resolve().unwrap();
    dic2.compile(&mut cfgb.add_user()).unwrap();
    let jd2 = JapaneseDictionary::from_cfg(&cfgb.config()).unwrap();
    let tok = StatelessTokenizer::new(&jd2);
    let result = tok.tokenize("かぼすにいく", Mode::C, false).unwrap();
    assert_eq!(result.len(), 3);
    assert_eq!(result.get(0).word_id().dic(), 2);
    assert_eq!(result.get(0).part_of_speech()[0], "被子植物門");
}

#[test]
fn split_with_subset() {
    let mut cfgb = ConfigTestSupport::new();
    let mut dic = DictBuilder::new_system();
    dic.read_conn(super::MATRIX_10_10).unwrap();
    dic.read_lexicon(SYSTEM_LEX).unwrap();
    dic.resolve().unwrap();
    dic.compile(&mut cfgb.make_system()).unwrap();

    let jd = JapaneseDictionary::from_cfg(&cfgb.config()).unwrap();
    let mut tok = StatefulTokenizer::new(&jd, Mode::A);
    let mut res = MorphemeList::empty(&jd);
    tok.set_subset(InfoSubset::empty());
    tok.reset().push_str("東京都");
    tok.do_tokenize().unwrap();
    res.collect_results(&mut tok).unwrap();
    assert_eq!(res.len(), 2);
    //assert_eq!(res.get_end(0), 6);
    assert_eq!(res.get(1).end(), 9);
}
