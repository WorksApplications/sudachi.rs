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

use std::path::PathBuf;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use sudachi::analysis::stateful_tokenizer::StatefulTokenizer;
use sudachi::analysis::Mode;
use sudachi::config::Config;
use sudachi::dic::dictionary::JapaneseDictionary;
use sudachi::prelude::MorphemeList;
use sudachi::sentence_splitter::{SentenceSplitter, SplitSentences};

fn load_dictionary() -> JapaneseDictionary {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let config = Config::new(
        Some(manifest_dir.join("tests/resources/sudachi.json")),
        None,
        None,
    )
    .expect("test config should load");
    JapaneseDictionary::from_cfg(&config).expect("test dictionary should load")
}

fn prose_corpus_repeated(repeats: usize) -> String {
    [
        "吾輩は猫である。名前はまだ無い。",
        "どこで生れたかとんと見当がつかぬ。",
        "何でも薄暗いじめじめした所でニャーニャー泣いていた事だけは記憶している。",
        "この文章は、括弧（たとえば「引用。まだ続く」）を含む。",
    ]
    .join("")
    .repeat(repeats)
}

fn prose_corpus() -> String {
    prose_corpus_repeated(512)
}

fn punctuation_heavy_corpus() -> String {
    [
        "え？！本当に？！",
        "価格は3.141ではない。四百十.〇も数字として扱う。",
        "箇条書き1.と2.の途中では切らない。",
        "「まだ終わらない。引用の内側だ」。ここで終わる。",
        "あ・・・？！次へ。",
    ]
    .join("")
    .repeat(512)
}

fn long_line_corpus() -> String {
    let mut text = "長い文節".repeat(1200);
    text.push(' ');
    text.push_str(&"さらに長い文節".repeat(600));
    text.push('。');
    text
}

fn html_corpus() -> String {
    [
        "一つ目の段落<br><br>二つ目の段落。",
        "大文字の区切り<BR><BR>次の段落。",
        "単独の<br>タグでは切らない。",
    ]
    .join("")
    .repeat(512)
}

fn corpora() -> Vec<(&'static str, String)> {
    vec![
        ("prose", prose_corpus()),
        ("punctuation_heavy", punctuation_heavy_corpus()),
        ("long_line", long_line_corpus()),
        ("html", html_corpus()),
    ]
}

fn bench_split_only(c: &mut Criterion) {
    let dict = load_dictionary();
    let splitter = SentenceSplitter::new().with_checker(dict.lexicon());

    let mut group = c.benchmark_group("sentence_splitter_only");
    for (name, corpus) in corpora() {
        group.throughput(Throughput::Bytes(corpus.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(name), &corpus, |b, input| {
            b.iter(|| {
                let total_len = splitter
                    .split(black_box(input))
                    .map(|(_, sentence)| sentence.len())
                    .sum::<usize>();
                black_box(total_len)
            })
        });
    }
    group.finish();
}

fn bench_end_to_end(c: &mut Criterion) {
    let dict = load_dictionary();
    let splitter = SentenceSplitter::new().with_checker(dict.lexicon());
    let mut tokenizer = StatefulTokenizer::create(&dict, false, Mode::C);
    let mut morphemes = MorphemeList::empty(&dict);
    let input = prose_corpus_repeated(128);

    let mut group = c.benchmark_group("sentence_splitter_end_to_end");
    group.throughput(Throughput::Bytes(input.len() as u64));

    group.bench_function("split_sentences_yes", |b| {
        b.iter(|| {
            let mut total = 0;
            for (_, sentence) in splitter.split(black_box(&input)) {
                tokenizer.reset().push_str(sentence);
                tokenizer
                    .do_tokenize()
                    .expect("tokenization should succeed");
                morphemes
                    .collect_results(&mut tokenizer)
                    .expect("collection should succeed");
                total += morphemes.len();
            }
            black_box(total)
        })
    });

    group.bench_function("split_sentences_none", |b| {
        b.iter(|| {
            tokenizer.reset().push_str(black_box(&input));
            tokenizer
                .do_tokenize()
                .expect("tokenization should succeed");
            morphemes
                .collect_results(&mut tokenizer)
                .expect("collection should succeed");
            black_box(morphemes.len())
        })
    });

    group.finish();
}

criterion_group!(benches, bench_split_only, bench_end_to_end);
criterion_main!(benches);
