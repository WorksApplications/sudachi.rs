#[macro_use]
extern crate lazy_static;

mod common;
use common::{GRAMMAR, TOKENIZER};

#[test]
fn get_part_of_speech_size() {
    // pos from system test dict
    assert_eq!(8, GRAMMAR.pos_list.len());

    // user test dict contains another pos
    assert_eq!(9, TOKENIZER.grammar.pos_list.len());
}

#[test]
fn get_part_of_speech_string() {
    let pos = &GRAMMAR.pos_list[0];
    assert!(!pos.is_empty());
    assert_eq!("助動詞", pos[0]);
}

// todo: not implemented in python
// fn creat_with_merging_settings
// fn creat_with_merging_null_settings
