#[macro_use]
extern crate lazy_static;

extern crate sudachi;
use sudachi::prelude::*;

mod common;
use common::TOKENIZER;

/// Expected chunks for a text in a given mode
#[derive(Debug, Clone)]
struct ChunkExpectation<'a> {
    a: &'a [&'a str],
    b: &'a [&'a str],
    c: &'a [&'a str],
}

/// Get morpheme list from text
fn tokenize(text: &str, mode: Mode) -> Vec<Morpheme> {
    TOKENIZER
        .tokenize(text, mode, false)
        .expect("Failed to get tokens")
}

/// Get text chunks from text
fn tokenized_chunks(text: &str, mode: Mode) -> Vec<String> {
    tokenize(text, mode)
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

#[test]
fn tokenize_small_katakana_only() {
    let ms = tokenize("ァ", Mode::C);
    assert_eq!(1, ms.len());
}

#[test]
fn get_word_id() {
    let ms = tokenize("京都", Mode::C);
    assert_eq!(1, ms.len());
    let pos = ms[0].pos().expect("failed to get pos");
    assert_eq!(["名詞", "固有名詞", "地名", "一般", "*", "*"], &pos[..]);

    // we do not have word_id field in Morpheme and skip testing.
    // todo: this fails since default (not for test) config file is used
    let ms = tokenize("ぴらる", Mode::C);
    assert_eq!(1, ms.len());
    let pos = ms[0].pos().expect("failed to get pos");
    assert_eq!(["名詞", "普通名詞", "一般", "*", "*", "*"], &pos[..]);
}

#[test]
fn get_dictionary_id() {
    // todo: this fails since default (not for test) config file is used

    let ms = tokenize("京都", Mode::C);
    assert_eq!(1, ms.len());
    assert_eq!(0, ms[0].word_info.dictionary_form_word_id);

    let ms = tokenize("ぴらる", Mode::C);
    assert_eq!(1, ms.len());
    assert_eq!(1, ms[0].word_info.dictionary_form_word_id);

    let ms = tokenize("京", Mode::C);
    assert_eq!(1, ms.len());
    assert!(ms[0].word_info.dictionary_form_word_id < 0);
}

#[test]
fn get_synonym_group_id() {
    // todo: after dictionary version update
}

#[test]
fn tokenize_kanji_alphabet_word() {
    // todo: this fails since default (not for test) config file is used
    assert_eq!(1, tokenize("特a", Mode::C).len());
    assert_eq!(1, tokenize("ab", Mode::C).len());
    assert_eq!(2, tokenize("特ab", Mode::C).len());
}

#[test]
fn tokenize_with_dots() {
    let ms = tokenize("京都…", Mode::C);
    assert_eq!(4, ms.len());
    assert_eq!("…", ms[1].surface());
    assert_eq!(".", ms[1].normalized_form());
    assert_eq!("", ms[2].surface());
    assert_eq!(".", ms[2].normalized_form());
    assert_eq!("", ms[3].surface());
    assert_eq!(".", ms[3].normalized_form());
}
