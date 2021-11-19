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

use crate::dic::build::lexicon::LexiconReader;
use crate::dic::build::primitives::Utf16Writer;
use crate::dic::read::word_info::WordInfoParser;
use crate::dic::subset::InfoSubset;
use crate::dic::word_id::WordId;

#[test]
fn wordinfo_subset_surface() {
    let data = make_data();
    let wi = WordInfoParser::subset(InfoSubset::SURFACE)
        .parse(&data)
        .unwrap();
    assert_eq!(wi.surface, "京都");
}

#[test]
fn wordinfo_subset_len() {
    let data = make_data();
    let wi = WordInfoParser::subset(InfoSubset::HEAD_WORD_LENGTH)
        .parse(&data)
        .unwrap();
    assert_eq!(wi.head_word_length, 6);
}

#[test]
fn wordinfo_subset_pos() {
    let data = make_data();
    let wi = WordInfoParser::subset(InfoSubset::POS_ID)
        .parse(&data)
        .unwrap();
    assert_eq!(wi.pos_id, 1);
}

#[test]
fn wordinfo_subset_norm() {
    let data = make_data();
    let wi = WordInfoParser::subset(InfoSubset::NORMALIZED_FORM)
        .parse(&data)
        .unwrap();
    assert_eq!(wi.normalized_form, "東京");
}

#[test]
fn wordinfo_subset_reading() {
    let data = make_data();
    let wi = WordInfoParser::subset(InfoSubset::READING_FORM)
        .parse(&data)
        .unwrap();
    assert_eq!(wi.reading_form, "キョウト");
}

#[test]
fn wordinfo_subset_dic_form_id() {
    let data = make_data();
    let wi = WordInfoParser::subset(InfoSubset::DIC_FORM_WORD_ID)
        .parse(&data)
        .unwrap();
    assert_eq!(wi.dictionary_form_word_id, 1);
}

#[test]
fn wordinfo_subset_dic_split_a() {
    let data = make_data();
    let wi = WordInfoParser::subset(InfoSubset::SPLIT_A)
        .parse(&data)
        .unwrap();
    assert_eq!(wi.a_unit_split, [WordId::new(0, 1), WordId::new(0, 2)]);
}

#[test]
fn wordinfo_subset_dic_split_b() {
    let data = make_data();
    let wi = WordInfoParser::subset(InfoSubset::SPLIT_B)
        .parse(&data)
        .unwrap();
    assert_eq!(wi.b_unit_split, [WordId::new(0, 3), WordId::new(0, 4)]);
}

#[test]
fn wordinfo_subset_dic_word_structure() {
    let data = make_data();
    let wi = WordInfoParser::subset(InfoSubset::WORD_STRUCTURE)
        .parse(&data)
        .unwrap();
    assert_eq!(
        wi.word_structure,
        vec![WordId::new(0, 5), WordId::new(0, 6)]
    );
}

#[test]
fn wordinfo_subset_dic_synonym() {
    let data = make_data();
    let wi = WordInfoParser::subset(InfoSubset::SYNONYM_GROUP_ID)
        .parse(&data)
        .unwrap();
    assert_eq!(wi.synonym_group_ids, [7, 8]);
}

fn make_data() -> Vec<u8> {
    let mut rdr = LexiconReader::new();
    let data: &[u8] = include_bytes!("data_full_wordinfo.csv");
    rdr.read_bytes(data).unwrap();
    let mut u16w = Utf16Writer::new();
    let mut data: Vec<u8> = Vec::new();
    rdr.entries[1]
        .write_word_info(&mut u16w, &mut data)
        .unwrap();
    data
}
