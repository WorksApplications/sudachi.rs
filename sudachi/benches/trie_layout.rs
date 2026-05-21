/*
 *  Copyright (c) 2026 Works Applications Co., Ltd.
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

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use sudachi::dic::binary_loader::BinaryDictionary;
use sudachi::dic::build::{CacheAwareOptions, DictBuilder, TrieBuildStrategy, TrieProfileMode};
use tempfile::NamedTempFile;

const MATRIX: &[u8] = include_bytes!("../src/dic/build/test/matrix_10x10.def");
const LEXICON: &[u8] = include_bytes!("../src/dic/build/test/lex.csv");
const DEFAULT_INPUTS: &[&str] = &[
    "東京都に行く",
    "京都府",
    "東京湾",
    "アイアイウ",
    "六三四",
    "01234567890123456789",
    "存在しない語",
];

struct BenchFixture {
    matrix: Vec<u8>,
    lexicons: Vec<Vec<u8>>,
    inputs: BenchInputs,
    external_profile_path: PathBuf,
    _generated_profile: Option<NamedTempFile>,
}

struct BenchInputs {
    hits: Vec<String>,
    misses: Vec<String>,
    mixed: Vec<String>,
}

#[derive(Clone)]
struct Variant {
    name: &'static str,
    strategy: TrieBuildStrategy,
}

struct CompiledVariant {
    name: &'static str,
    dict_bytes: usize,
    trie_bytes: usize,
    dictionary: BinaryDictionary<'static>,
}

fn fixture() -> BenchFixture {
    let matrix = read_env_bytes("SUDACHI_TRIE_BENCH_MATRIX", MATRIX);
    let lexicons = read_lexicons();
    let inputs = build_inputs(&lexicons);
    let (external_profile_path, generated_profile) = external_profile_path(&inputs.hits);

    BenchFixture {
        matrix,
        lexicons,
        inputs,
        external_profile_path,
        _generated_profile: generated_profile,
    }
}

fn variants(external_profile_path: &Path) -> Vec<Variant> {
    vec![
        Variant {
            name: "classic_yada",
            strategy: TrieBuildStrategy::ClassicYada,
        },
        Variant {
            name: "cache_aware_uniform",
            strategy: TrieBuildStrategy::CacheAware(CacheAwareOptions {
                profile_mode: TrieProfileMode::Uniform,
                ..CacheAwareOptions::default()
            }),
        },
        Variant {
            name: "cache_aware_prefix",
            strategy: TrieBuildStrategy::CacheAware(CacheAwareOptions::default()),
        },
        Variant {
            name: "cache_aware_external_profile",
            strategy: TrieBuildStrategy::CacheAware(CacheAwareOptions {
                profile_mode: TrieProfileMode::ExternalKeyProfile(external_profile_path.to_owned()),
                ..CacheAwareOptions::default()
            }),
        },
    ]
}

fn read_env_bytes(env_name: &str, fallback: &[u8]) -> Vec<u8> {
    env::var_os(env_name)
        .map(|path| fs::read(path).expect("failed to read benchmark input file"))
        .unwrap_or_else(|| fallback.to_vec())
}

fn read_lexicons() -> Vec<Vec<u8>> {
    if let Some(paths) = env::var_os("SUDACHI_TRIE_BENCH_LEXICONS") {
        return env::split_paths(&paths)
            .map(|path| fs::read(path).expect("failed to read benchmark lexicon"))
            .collect();
    }
    if let Some(path) = env::var_os("SUDACHI_TRIE_BENCH_LEXICON") {
        return vec![fs::read(path).expect("failed to read benchmark lexicon")];
    }
    vec![LEXICON.to_vec()]
}

fn build_inputs(lexicons: &[Vec<u8>]) -> BenchInputs {
    let mut hits = index_forms(lexicons);
    hits.truncate(256);
    if hits.is_empty() {
        hits.extend(DEFAULT_INPUTS.iter().map(|s| (*s).to_owned()));
    }

    let misses: Vec<String> = hits
        .iter()
        .take(256)
        .map(|input| format!("{input}\u{e000}"))
        .collect();

    let mixed = env::var_os("SUDACHI_TRIE_BENCH_INPUTS")
        .map(|path| read_inputs(PathBuf::from(path)))
        .unwrap_or_else(|| {
            let mut inputs: Vec<String> = DEFAULT_INPUTS.iter().map(|s| (*s).to_owned()).collect();
            inputs.extend(hits.iter().take(64).cloned());
            inputs.extend(misses.iter().take(64).cloned());
            dedup(inputs)
        });

    BenchInputs {
        hits,
        misses,
        mixed,
    }
}

fn read_inputs(path: PathBuf) -> Vec<String> {
    fs::read_to_string(path)
        .expect("failed to read benchmark inputs")
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(ToOwned::to_owned)
        .collect()
}

fn index_forms(lexicons: &[Vec<u8>]) -> Vec<String> {
    let mut forms = Vec::new();
    for lexicon in lexicons {
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_reader(lexicon.as_slice());
        let index_col = reader
            .headers()
            .ok()
            .and_then(|headers| {
                headers
                    .iter()
                    .position(|header| header.eq_ignore_ascii_case("index_form"))
            })
            .unwrap_or(0);

        for record in reader.records().flatten() {
            let Some(form) = record.get(index_col) else {
                continue;
            };
            if !form.is_empty() {
                forms.push(form.to_owned());
            }
        }
    }
    dedup(forms)
}

fn dedup(inputs: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    inputs
        .into_iter()
        .filter(|input| seen.insert(input.clone()))
        .collect()
}

fn external_profile_path(hits: &[String]) -> (PathBuf, Option<NamedTempFile>) {
    if let Some(path) = env::var_os("SUDACHI_TRIE_BENCH_PROFILE") {
        return (PathBuf::from(path), None);
    }

    let mut file = NamedTempFile::new().expect("failed to create benchmark profile");
    let max = hits.len().min(512);
    for (rank, input) in hits.iter().take(max).enumerate() {
        write_profile_key(&mut file, input.as_bytes());
        writeln!(file, "\t{}", max - rank).expect("failed to write benchmark profile");
    }
    (file.path().to_owned(), Some(file))
}

fn write_profile_key<W: Write>(writer: &mut W, bytes: &[u8]) {
    writer
        .write_all(b"hex:")
        .expect("failed to write benchmark profile");
    for byte in bytes {
        write!(writer, "{byte:02X}").expect("failed to write benchmark profile");
    }
}

fn compile(fixture: &BenchFixture, strategy: TrieBuildStrategy) -> Vec<u8> {
    let mut builder = DictBuilder::new_system();
    builder.set_trie_build_strategy(strategy);
    builder.read_conn(fixture.matrix.as_slice()).unwrap();
    for lexicon in &fixture.lexicons {
        builder.read_lexicon(lexicon.as_slice()).unwrap();
    }
    builder.resolve().unwrap();

    let mut bytes = Vec::new();
    builder.compile(&mut bytes).unwrap();
    bytes
}

fn compile_variants(fixture: &BenchFixture) -> Vec<CompiledVariant> {
    let mut compiled = Vec::new();
    for variant in variants(&fixture.external_profile_path) {
        let bytes = compile(fixture, variant.strategy);
        let dict_bytes = bytes.len();
        let leaked: &'static [u8] = Box::leak(bytes.into_boxed_slice());
        let dictionary = BinaryDictionary::load_system(leaked).unwrap();
        let trie_bytes = dictionary.lexicon.trie.total_size();
        compiled.push(CompiledVariant {
            name: variant.name,
            dict_bytes,
            trie_bytes,
            dictionary,
        });
    }
    validate_variants(&compiled, &fixture.inputs);
    print_size_report(&compiled);
    compiled
}

fn validate_variants(compiled: &[CompiledVariant], inputs: &BenchInputs) {
    let Some(baseline) = compiled.first() else {
        return;
    };
    for input in all_inputs(inputs) {
        let expected = trie_entries(baseline, &input);
        for variant in compiled.iter().skip(1) {
            let actual = trie_entries(variant, &input);
            assert_eq!(
                actual, expected,
                "variant {} disagrees with classic_yada for input {input:?}",
                variant.name
            );
        }
    }
}

fn all_inputs(inputs: &BenchInputs) -> Vec<String> {
    let mut result = Vec::new();
    result.extend(inputs.hits.iter().cloned());
    result.extend(inputs.misses.iter().cloned());
    result.extend(inputs.mixed.iter().cloned());
    dedup(result)
}

fn trie_entries(variant: &CompiledVariant, input: &str) -> Vec<(u32, usize)> {
    variant
        .dictionary
        .lexicon
        .trie
        .common_prefix_iterator(input.as_bytes(), 0)
        .map(|entry| (entry.value, entry.end))
        .collect()
}

fn print_size_report(compiled: &[CompiledVariant]) {
    eprintln!("trie_layout/size_report");
    for variant in compiled {
        eprintln!(
            "  {:30} dict_bytes={} trie_bytes={}",
            variant.name, variant.dict_bytes, variant.trie_bytes
        );
    }
}

fn bench_trie_build(c: &mut Criterion) {
    let fixture = fixture();
    let mut group = c.benchmark_group("trie_layout/build");
    for variant in variants(&fixture.external_profile_path) {
        group.bench_with_input(
            BenchmarkId::from_parameter(variant.name),
            &variant.strategy,
            |b, strategy| {
                b.iter(|| black_box(compile(&fixture, strategy.clone())));
            },
        );
    }
    group.finish();
}

fn bench_common_prefix(c: &mut Criterion) {
    let fixture = fixture();
    let compiled = compile_variants(&fixture);

    bench_input_set(
        c,
        "trie_layout/common_prefix_hit",
        &compiled,
        &fixture.inputs.hits,
    );
    bench_input_set(
        c,
        "trie_layout/common_prefix_miss",
        &compiled,
        &fixture.inputs.misses,
    );
    bench_input_set(
        c,
        "trie_layout/common_prefix_mixed",
        &compiled,
        &fixture.inputs.mixed,
    );
}

fn bench_input_set(
    c: &mut Criterion,
    group_name: &'static str,
    compiled: &[CompiledVariant],
    inputs: &[String],
) {
    let mut group = c.benchmark_group(group_name);
    group.throughput(criterion::Throughput::Elements(inputs.len() as u64));
    for variant in compiled {
        group.bench_function(variant.name, |b| {
            b.iter(|| {
                let mut total = 0usize;
                for input in inputs {
                    total += variant
                        .dictionary
                        .lexicon
                        .trie
                        .common_prefix_iterator(input.as_bytes(), 0)
                        .count();
                }
                black_box(total);
            })
        });
    }
    group.finish();
}

criterion_group!(benches, bench_trie_build, bench_common_prefix);
criterion_main!(benches);
