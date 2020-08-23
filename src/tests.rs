//! Crate tests

use std::env;
use std::fs::{self, File};
use std::io::Read;

use crate::prelude::*;

lazy_static! {
    static ref DICTIONARY_BYTES: Vec<u8> = {
        let dictionary_path = env::var_os("SUDACHI_DICT_PATH").expect("Must set env var SUDACHI_DICT_PATH with path to Sudachi dictionary (relative to current dir)");
        let dictionary_stat = fs::metadata(&dictionary_path).expect("Unable to stat dictionary");
        let mut dictionary_file = File::open(dictionary_path).expect("Unable to open dictionary");
        let mut storage_buf = Vec::with_capacity(dictionary_stat.len() as usize);
        dictionary_file
            .read_to_end(&mut storage_buf)
            .expect("Failed to read from dictionary file");
        storage_buf
    };
    static ref TOKENIZER: Tokenizer<'static> = Tokenizer::from_dictionary_bytes(&DICTIONARY_BYTES)
        .expect("Failed to create Tokenizer for tests");
}

/// Expected chunks for a text in a given mode
#[derive(Debug, Clone)]
struct ChunkExpectation<'a> {
    a: &'a [&'a str],
    b: &'a [&'a str],
    c: &'a [&'a str],
}

/// Get text chunks from text
fn tokenized_chunks(text: &str, mode: Mode) -> Vec<String> {
    let tokens = TOKENIZER.tokenize(text, mode, false);
    tokens
        .iter()
        .map(|tok| tok.surface().clone())
        .collect::<Vec<String>>()
}

fn assert_tokenized_chunk_modes(text: &str, expected_chunks: ChunkExpectation) {
    assert_eq!(
        tokenized_chunks(text, Mode::A),
        expected_chunks.a,
        "failed for mode A, text={:?}",
        text
    );
    assert_eq!(
        tokenized_chunks(text, Mode::B),
        expected_chunks.b,
        "failed for mode B, text={:?}",
        text
    );
    assert_eq!(
        tokenized_chunks(text, Mode::C),
        expected_chunks.c,
        "failed for mode C, text={:?}",
        text
    );
}

#[test]
fn chunks() {
    assert_tokenized_chunk_modes(
        "選挙管理委員会",
        ChunkExpectation {
            a: &["選挙", "管理", "委員", "会"],
            b: &["選挙", "管理", "委員会"],
            c: &["選挙管理委員会"],
        },
    );

    assert_tokenized_chunk_modes(
        "客室乗務員",
        ChunkExpectation {
            a: &["客室", "乗務", "員"],
            b: &["客室", "乗務員"],
            c: &["客室乗務員"],
        },
    );

    assert_tokenized_chunk_modes(
        "労働者協同組合",
        ChunkExpectation {
            a: &["労働", "者", "協同", "組合"],
            b: &["労働者", "協同", "組合"],
            c: &["労働者協同組合"],
        },
    );

    assert_tokenized_chunk_modes(
        "機能性食品",
        ChunkExpectation {
            a: &["機能", "性", "食品"],
            b: &["機能性", "食品"],
            c: &["機能性食品"],
        },
    );
}
