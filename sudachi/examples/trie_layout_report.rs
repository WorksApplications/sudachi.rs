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

use std::collections::HashSet;
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use sudachi::dic::binary_loader::BinaryDictionary;
use sudachi::dic::build::{CacheAwareOptions, DictBuilder, TrieBuildStrategy, TrieProfileMode};
use tempfile::NamedTempFile;

const MATRIX: &[u8] = include_bytes!("../src/dic/build/test/matrix_10x10.def");
const LEXICON: &[u8] = include_bytes!("../src/dic/build/test/lex.csv");
const LOOKUP_ROUNDS: usize = 20_000;
const DEFAULT_INPUT_LIMIT: usize = 256;

struct Fixture {
    matrix: Vec<u8>,
    lexicons: Vec<Vec<u8>>,
    inputs: Inputs,
    profile_path: PathBuf,
    _generated_profile: Option<NamedTempFile>,
}

struct Inputs {
    hits: Vec<String>,
    misses: Vec<String>,
    mixed: Vec<String>,
}

#[derive(Clone)]
struct Variant {
    name: &'static str,
    strategy: TrieBuildStrategy,
}

struct Built {
    name: &'static str,
    build_time: Duration,
    dict_bytes: usize,
    trie_bytes: usize,
    dictionary: BinaryDictionary<'static>,
}

fn main() {
    let fixture = fixture();
    let variants = variants(&fixture.profile_path);
    let mut built = Vec::new();

    for variant in variants {
        let started = Instant::now();
        let bytes = compile(&fixture, variant.strategy);
        let build_time = started.elapsed();
        let dict_bytes = bytes.len();
        let leaked: &'static [u8] = Box::leak(bytes.into_boxed_slice());
        let dictionary = BinaryDictionary::load_system(leaked).unwrap();
        let trie_bytes = dictionary.lexicon.trie.total_size();
        built.push(Built {
            name: variant.name,
            build_time,
            dict_bytes,
            trie_bytes,
            dictionary,
        });
    }

    validate(&built, &fixture.inputs);
    print_report(&built, &fixture.inputs);
}

fn fixture() -> Fixture {
    let matrix = read_env_bytes("SUDACHI_TRIE_BENCH_MATRIX", MATRIX);
    let lexicons = read_lexicons();
    let inputs = build_inputs(&lexicons);
    let (profile_path, generated_profile) = external_profile_path(&inputs.hits);
    Fixture {
        matrix,
        lexicons,
        inputs,
        profile_path,
        _generated_profile: generated_profile,
    }
}

fn variants(profile_path: &Path) -> Vec<Variant> {
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
                profile_mode: TrieProfileMode::ExternalKeyProfile(profile_path.to_owned()),
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

fn build_inputs(lexicons: &[Vec<u8>]) -> Inputs {
    let limit = env::var("SUDACHI_TRIE_REPORT_INPUT_LIMIT")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(DEFAULT_INPUT_LIMIT);
    let mut hits = sample_inputs(index_forms(lexicons), limit);
    if hits.is_empty() {
        hits.push("東京都".to_owned());
    }

    let misses: Vec<String> = hits
        .iter()
        .map(|input| format!("{input}\u{e000}"))
        .collect();
    let mixed = env::var_os("SUDACHI_TRIE_BENCH_INPUTS")
        .map(|path| read_inputs(PathBuf::from(path), limit))
        .unwrap_or_else(|| {
            let mut inputs = Vec::new();
            inputs.extend(hits.iter().take(limit / 2).cloned());
            inputs.extend(misses.iter().take(limit / 2).cloned());
            dedup(inputs)
        });

    Inputs {
        hits,
        misses,
        mixed,
    }
}

fn read_inputs(path: PathBuf, limit: usize) -> Vec<String> {
    fs::read_to_string(path)
        .expect("failed to read benchmark inputs")
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .take(limit)
        .map(ToOwned::to_owned)
        .collect()
}

fn index_forms(lexicons: &[Vec<u8>]) -> Vec<String> {
    let mut forms = Vec::new();
    for lexicon in lexicons {
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_reader(lexicon.as_slice());
        for record in reader.records().flatten() {
            let Some(form) = record.get(0) else {
                continue;
            };
            if !form.is_empty() {
                forms.push(form.to_owned());
            }
        }
    }
    dedup(forms)
}

fn sample_inputs(mut inputs: Vec<String>, limit: usize) -> Vec<String> {
    inputs.sort();
    inputs.dedup();
    if inputs.len() <= limit {
        return inputs;
    }

    (0..limit)
        .map(|i| {
            let index = i * inputs.len() / limit;
            inputs[index].clone()
        })
        .collect()
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
    let max = hits.len();
    for (rank, input) in hits.iter().enumerate() {
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

fn compile(fixture: &Fixture, strategy: TrieBuildStrategy) -> Vec<u8> {
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

fn validate(built: &[Built], inputs: &Inputs) {
    let baseline = &built[0];
    for input in all_inputs(inputs) {
        let expected = trie_entries(baseline, &input);
        for variant in built.iter().skip(1) {
            let actual = trie_entries(variant, &input);
            assert_eq!(
                actual, expected,
                "variant {} disagrees with classic_yada for input {input:?}",
                variant.name
            );
        }
    }
}

fn all_inputs(inputs: &Inputs) -> Vec<String> {
    let mut result = Vec::new();
    result.extend(inputs.hits.iter().cloned());
    result.extend(inputs.misses.iter().cloned());
    result.extend(inputs.mixed.iter().cloned());
    dedup(result)
}

fn trie_entries(variant: &Built, input: &str) -> Vec<(u32, usize)> {
    variant
        .dictionary
        .lexicon
        .trie
        .common_prefix_iterator(input.as_bytes(), 0)
        .map(|entry| (entry.value, entry.end))
        .collect()
}

fn print_report(built: &[Built], inputs: &Inputs) {
    println!(
        "variant\tbuild_ms\tdict_bytes\ttrie_bytes\thit_ns_per_batch\tmiss_ns_per_batch\tmixed_ns_per_batch\thit_entries\tmiss_entries\tmixed_entries"
    );
    for variant in built {
        let (hit_ns, hit_entries) = measure_lookup(variant, &inputs.hits);
        let (miss_ns, miss_entries) = measure_lookup(variant, &inputs.misses);
        let (mixed_ns, mixed_entries) = measure_lookup(variant, &inputs.mixed);
        println!(
            "{}\t{:.3}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            variant.name,
            variant.build_time.as_secs_f64() * 1000.0,
            variant.dict_bytes,
            variant.trie_bytes,
            hit_ns,
            miss_ns,
            mixed_ns,
            hit_entries,
            miss_entries,
            mixed_entries
        );
    }
}

fn measure_lookup(variant: &Built, inputs: &[String]) -> (u128, usize) {
    let started = Instant::now();
    let mut total = 0usize;
    for _ in 0..LOOKUP_ROUNDS {
        for input in inputs {
            total += variant
                .dictionary
                .lexicon
                .trie
                .common_prefix_iterator(input.as_bytes(), 0)
                .count();
        }
    }
    let per_batch = started.elapsed().as_nanos() / LOOKUP_ROUNDS as u128;
    (per_batch, total)
}
