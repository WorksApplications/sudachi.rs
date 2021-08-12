use std::env;

extern crate sudachi;
use sudachi::dic::{grammar::Grammar, header::Header, lexicon::Lexicon};
use sudachi::prelude::*;

lazy_static! {
    static ref DICTIONARY_BYTES: Vec<u8> = {
        let dictionary_path = env::var_os("SUDACHI_DICT_PATH").expect("Must set env var SUDACHI_DICT_PATH with path to Sudachi dictionary (relative to current dir)");
        let dictionary_bytes = dictionary_bytes_from_path(dictionary_path)
            .expect("Failed to read dictionary from path");
        dictionary_bytes
    };
    pub static ref HEADER: Header =
        Header::new(&DICTIONARY_BYTES).expect("Failed to create Header for tests");
    pub static ref GRAMMAR: Grammar<'static> =
        Grammar::new(&DICTIONARY_BYTES, Header::STORAGE_SIZE)
            .expect("Failed to create Grammar for tests");
    pub static ref LEXICON: &'static Lexicon<'static> = &TOKENIZER.lexicon;
    pub static ref TOKENIZER: Tokenizer<'static> =
        Tokenizer::from_dictionary_bytes(&DICTIONARY_BYTES)
            .expect("Failed to create Tokenizer for tests");
}
