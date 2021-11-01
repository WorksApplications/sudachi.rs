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

use crate::analysis::stateless_tokenizer::DictionaryAccess;
use crate::dic::build::DictBuilder;
use crate::dic::grammar::Grammar;
use crate::dic::header::{HeaderVersion, SystemDictVersion};
use crate::dic::lexicon::{Lexicon, LexiconEntry};
use crate::dic::word_id::WordId;
use crate::dic::DictionaryLoader;
use unicode_normalization::__test_api::quick_check::IsNormalized::No;

#[test]
fn build_grammar() {
    let mut bldr = DictBuilder::new();
    bldr.read_conn(include_bytes!("matrix_10x10.def")).unwrap();
    assert_eq!(
        1,
        bldr.read_lexicon(include_bytes!("data_1word.csv")).unwrap()
    );
    let mut built = Vec::new();
    let written = bldr.write_grammar(&mut built).unwrap();
    assert_eq!(built.len(), written);
    let grammar = Grammar::parse(&built, 0).unwrap();
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
    let mut bldr = DictBuilder::new();
    assert_eq!(
        1,
        bldr.read_lexicon(include_bytes!("data_1word.csv")).unwrap()
    );
    let mut built = Vec::new();
    bldr.write_lexicon(&mut built, 0).unwrap();
    let mut lex = Lexicon::parse(&built, 0, true).unwrap();
    lex.set_dic_id(0);
    let mut iter = lex.lookup("京都".as_bytes(), 0);
    assert_eq!(
        iter.next(),
        Some(LexiconEntry {
            word_id: WordId::new(0, 0),
            end: 6
        })
    );
    assert_eq!(iter.next(), None);
    assert_eq!((6, 6, 5293), lex.get_word_param(0).unwrap());
    let wi = lex.get_word_info(0).unwrap();
    assert_eq!(wi.surface, "京都");
    assert_eq!(wi.normalized_form, "京都");
    assert_eq!(wi.dictionary_form, "京都");
    assert_eq!(wi.reading_form, "キョウト");
}

#[test]
fn build_system_1word() {
    let mut bldr = DictBuilder::new();
    bldr.read_conn(include_bytes!("matrix_10x10.def")).unwrap();
    assert_eq!(
        1,
        bldr.read_lexicon(include_bytes!("data_1word.csv")).unwrap()
    );
    let mut built = Vec::new();
    bldr.compile(&mut built).unwrap();
    let dic = DictionaryLoader::read_dictionary(&built).unwrap();
    assert_eq!(
        dic.header.version,
        HeaderVersion::SystemDict(SystemDictVersion::Version2)
    );

    let dic = dic.to_loaded().unwrap();

    let entry = dic.lexicon().lookup("京都".as_bytes(), 0).next().unwrap();
    assert_eq!(entry.word_id, WordId::new(0, 0));
    let info = dic.lexicon().get_word_info(entry.word_id).unwrap();
    assert_eq!(info.surface, "京都");
    assert_eq!(info.reading_form, "キョウト");
}

#[test]
fn build_system_3words() {
    let mut bldr = DictBuilder::new();
    bldr.read_conn(include_bytes!("matrix_10x10.def")).unwrap();
    assert_eq!(
        3,
        bldr.read_lexicon(include_bytes!("data_3words.csv"))
            .unwrap()
    );
    bldr.resolve().unwrap();
    let mut built = Vec::new();
    bldr.compile(&mut built).unwrap();
    let dic = DictionaryLoader::read_dictionary(&built).unwrap();
    let dic = dic.to_loaded().unwrap();
    let mut iter = dic.lexicon().lookup("東京".as_bytes(), 0);
    let entry = iter.next().unwrap();
    assert_eq!(entry.word_id, WordId::new(0, 1));
    let entry = iter.next().unwrap();
    assert_eq!(entry.word_id, WordId::new(0, 2));
    assert_eq!(iter.next(), None);
    let info = dic.lexicon().get_word_info(entry.word_id).unwrap();
    assert_eq!(info.a_unit_split, [WordId::new(0, 1), WordId::new(0, 0)]);
}
