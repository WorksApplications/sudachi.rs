/*
 * Copyright (c) 2026 Works Applications Co., Ltd.
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

//! Differential parity for the pipelined + prefetched lattice lookup: it must
//! produce identical tokenization to the scalar path.

extern crate sudachi;

use sudachi::prelude::Mode;

mod common;
use crate::common::TestStatefulTokenizer;

/// Snapshot of one result: the fields that pin the exact lattice path.
type Snapshot = Vec<(usize, usize, String, Vec<String>, String, String)>;

fn snapshot(tok: &mut TestStatefulTokenizer, text: &str) -> Snapshot {
    let ms = tok.tokenize(text);
    (0..ms.len())
        .map(|i| {
            let m = ms.get(i);
            let surface = m.surface().to_string();
            let pos = m.part_of_speech().to_vec();
            let normalized = m.normalized_form().to_string();
            let word_id = format!("{:?}", m.word_id());
            (m.begin(), m.end(), surface, pos, normalized, word_id)
        })
        .collect()
}

const SENTENCES: &[&str] = &[
    "",
    "京都",
    "東京都に住む",
    "すもももももももものうち",
    "10時に2人で会う。",
    "AIによる形態素解析は楽しい",
    "ABCＡＢＣ",
    "ぴらる",
    "外国人参政権について議論する",
    "きゃりーぱみゅぱみゅ",
    "プログラミング言語Rustで書かれた辞書",
    "！？「テスト」…",
    "東京特許許可局局長",
    "私はご飯を食べました。とても美味しかったです。",
    "アルゴリズムとデータ構造",
];

fn assert_parity_for_mode(mode: Mode) {
    let mut tok = TestStatefulTokenizer::new_built(mode);
    for &sentence in SENTENCES {
        tok.tok.set_pipelined_lookup(false);
        let scalar = snapshot(&mut tok, sentence);
        tok.tok.set_pipelined_lookup(true);
        let pipelined = snapshot(&mut tok, sentence);
        assert_eq!(
            scalar, pipelined,
            "pipelined lookup diverged from scalar for {sentence:?} in mode {mode:?}"
        );
    }
}

#[test]
fn pipelined_lookup_matches_scalar_mode_a() {
    assert_parity_for_mode(Mode::A);
}

#[test]
fn pipelined_lookup_matches_scalar_mode_b() {
    assert_parity_for_mode(Mode::B);
}

#[test]
fn pipelined_lookup_matches_scalar_mode_c() {
    assert_parity_for_mode(Mode::C);
}
