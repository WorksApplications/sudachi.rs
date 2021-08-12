#[macro_use]
extern crate lazy_static;

mod common;
use common::GRAMMAR;

#[test]
fn get_part_of_speech_size() {
    // todo: this fails since default (not for test) config file is used
    assert_eq!(9, GRAMMAR.pos_list.len());
}

#[test]
fn get_part_of_speech_string() {
    // todo: this fails since default (not for test) config file is used
    let pos = &GRAMMAR.pos_list[0];
    assert!(!pos.is_empty());
    assert_eq!("助動詞", pos[0]);
}

// fn creat_with_merging_settings
// fn creat_with_merging_null_settings
