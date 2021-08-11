use std::env;

#[macro_use]
extern crate lazy_static;

extern crate sudachi;
use sudachi::dic::{grammar::Grammar, header::Header};
use sudachi::prelude::*;

lazy_static! {
    static ref DICTIONARY_BYTES: Vec<u8> = {
        let dictionary_path = env::var_os("SUDACHI_DICT_PATH").expect("Must set env var SUDACHI_DICT_PATH with path to Sudachi dictionary (relative to current dir)");
        let dictionary_bytes = dictionary_bytes_from_path(dictionary_path)
            .expect("Failed to read dictionary from path");
        dictionary_bytes
    };
    static ref GRAMMAR: Grammar<'static> = Grammar::new(&DICTIONARY_BYTES, Header::STORAGE_SIZE)
        .expect("Failed to create Grammar for tests");
}

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
