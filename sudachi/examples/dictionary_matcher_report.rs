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

use crawdad::{MpTrie as CrawdadMpTrie, Trie as CrawdadTrie};
use daachorse::{CharwiseDoubleArrayAhoCorasick, DoubleArrayAhoCorasick};
use fst::Map as FstMap;
use rsmarisa::{Agent as MarisaAgent, Keyset as MarisaKeyset, Trie as MarisaTrie};
use std::collections::{HashMap, HashSet};
use std::env;
use std::error::Error;
use std::fs;
use std::hint::black_box;
use std::io::Write;
use std::mem::size_of;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use sudachi::dic::binary_loader::BinaryDictionary;
use sudachi::dic::build::{CacheAwareOptions, DictBuilder, TrieBuildStrategy, TrieProfileMode};
use tempfile::NamedTempFile;

const MATRIX: &[u8] = include_bytes!("../src/dic/build/test/matrix_10x10.def");
const LEXICON: &[u8] = include_bytes!("../src/dic/build/test/lex.csv");
const DEFAULT_ROUNDS: usize = 1_000;
const DEFAULT_INPUT_LIMIT: usize = 512;

type ReportResult<T> = Result<T, Box<dyn Error>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct PipelineMatch {
    start: usize,
    end: usize,
    value: u32,
}

#[derive(Clone, Copy, Debug, Default)]
struct StoredMatch {
    end: usize,
    value: u32,
}

#[derive(Clone)]
struct Entry {
    surface: String,
    value: u32,
}

struct Fixture {
    data_set: String,
    matrix: Vec<u8>,
    lexicons: Vec<Vec<u8>>,
    inputs: Vec<TextInput>,
    rounds: usize,
    profile_path: PathBuf,
    _generated_profile: Option<NamedTempFile>,
}

struct TextInput {
    text: String,
    starts: Vec<usize>,
    byte_to_start: Vec<usize>,
    byte_boundaries: Vec<bool>,
}

#[derive(Clone)]
struct YadaVariant {
    name: &'static str,
    strategy: TrieBuildStrategy,
}

struct BuiltMatcher {
    name: &'static str,
    family: &'static str,
    mode: MatchMode,
    build_time: Duration,
    heap_bytes: usize,
    serialized_bytes: usize,
    matcher: Matcher,
}

struct FailedMatcher {
    name: &'static str,
    family: &'static str,
    build_time: Duration,
    error: String,
}

enum Matcher {
    Yada(BinaryDictionary<'static>),
    DaachorseBytewise(DoubleArrayAhoCorasick<u32>),
    DaachorseCharwise(CharwiseDoubleArrayAhoCorasick<u32>),
    CrawdadTrie(CrawdadTrie),
    CrawdadMpTrie(CrawdadMpTrie),
    Fst(FstMap<Vec<u8>>),
    Marisa {
        trie: MarisaTrie,
        values: HashMap<Vec<u8>, u32>,
    },
}

#[derive(Clone, Copy)]
enum MatchMode {
    PrefixAllStarts,
    OnePassOverlapping,
}

#[derive(Clone, Copy)]
enum StorageKind {
    VecBucketsClearAll,
    FlatBucketsClearAll,
    FlatGenerationBuckets,
}

struct BenchmarkCase<'a> {
    matcher: &'a BuiltMatcher,
    storage: StorageKind,
}

struct ReportRow {
    variant: String,
    matcher: &'static str,
    family: &'static str,
    mode: &'static str,
    storage: &'static str,
    build_ms: f64,
    match_ns_per_batch: u128,
    match_ns_per_start: f64,
    speed_vs_current: f64,
    matcher_heap_bytes: usize,
    storage_heap_bytes: usize,
    total_heap_bytes: usize,
    total_heap_vs_current: f64,
    serialized_bytes: usize,
    same_order: bool,
    same_results: bool,
    mismatches: usize,
    checksum: u64,
}

struct Validation {
    same_order: bool,
    same_results: bool,
    mismatches: usize,
}

fn main() -> ReportResult<()> {
    let fixture = fixture();
    let mut built = build_yada_variants(&fixture)?;
    let entries = indexed_entries(&built[0], &fixture.lexicons)?;
    let (alternative_matchers, failed_matchers) = build_alternative_matchers(&entries)?;
    built.extend(alternative_matchers);

    let rows = report_rows(&built, &fixture.inputs, fixture.rounds);
    print_summary(&fixture);
    print_tsv(&rows, &failed_matchers);
    println!();
    print_markdown(&rows, &failed_matchers);
    Ok(())
}

fn fixture() -> Fixture {
    let (matrix, lexicons, data_set) = read_dictionary_fixture();
    let inputs = build_inputs(&lexicons);
    let (profile_path, generated_profile) = external_profile_path(&inputs);
    let rounds = env::var("SUDACHI_COMPARE_ROUNDS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(DEFAULT_ROUNDS);

    Fixture {
        data_set,
        matrix,
        lexicons,
        inputs,
        rounds,
        profile_path,
        _generated_profile: generated_profile,
    }
}

fn read_dictionary_fixture() -> (Vec<u8>, Vec<Vec<u8>>, String) {
    let data_set_name = env::var("SUDACHI_TRIE_BENCH_DATA_SET_NAME").ok();
    if let Some(paths) = env::var_os("SUDACHI_TRIE_BENCH_SURFACE_LEXICONS") {
        let forms = read_surface_forms(paths);
        let data_set = data_set_name
            .unwrap_or_else(|| format!("generated lexicon from {} index forms", forms.len()));
        return (MATRIX.to_vec(), vec![synthesize_lexicon(&forms)], data_set);
    }

    let matrix = read_env_bytes("SUDACHI_TRIE_BENCH_MATRIX", MATRIX);
    let lexicons = read_lexicons();
    let data_set = data_set_name.unwrap_or_else(|| {
        if env::var_os("SUDACHI_TRIE_BENCH_LEXICONS").is_some()
            || env::var_os("SUDACHI_TRIE_BENCH_LEXICON").is_some()
        {
            format!("provided lexicon files ({})", lexicons.len())
        } else {
            "built-in builder fixture".to_owned()
        }
    });
    (matrix, lexicons, data_set)
}

fn read_surface_forms(paths: std::ffi::OsString) -> Vec<String> {
    let mut forms = Vec::new();
    for path in env::split_paths(&paths) {
        let bytes = fs::read(&path).expect("failed to read surface source lexicon");
        forms.extend(index_forms(&[bytes]));
    }
    forms.sort();
    forms.dedup();
    if let Some(limit) = env_usize("SUDACHI_TRIE_BENCH_ENTRY_LIMIT") {
        forms = sample_inputs(forms, limit);
    }
    if forms.is_empty() {
        panic!("surface source lexicons did not contain any index forms");
    }
    forms
}

fn synthesize_lexicon(forms: &[String]) -> Vec<u8> {
    let mut writer = csv::WriterBuilder::new()
        .has_headers(false)
        .from_writer(Vec::new());
    writer
        .write_record([
            "IndexForm",
            "LeftId",
            "RightId",
            "Cost",
            "Headword",
            "POS1",
            "POS2",
            "POS3",
            "POS4",
            "POS5",
            "POS6",
            "Reading_Form",
            "Normalized_Form",
            "Dictionary_Form",
            "Split_A",
            "Split_B",
            "Split_C",
            "WordStructure",
            "SynonymGroups",
            "reference_id",
        ])
        .expect("failed to write generated lexicon header");
    for form in forms {
        writer
            .write_record([
                form.as_str(),
                "1",
                "1",
                "5000",
                "",
                "名詞",
                "普通名詞",
                "一般",
                "*",
                "*",
                "*",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
            ])
            .expect("failed to write generated lexicon row");
    }
    writer
        .into_inner()
        .expect("failed to finish generated lexicon")
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

fn build_inputs(lexicons: &[Vec<u8>]) -> Vec<TextInput> {
    let limit = env_usize("SUDACHI_COMPARE_INPUT_LIMIT").unwrap_or(DEFAULT_INPUT_LIMIT);
    let lines = env::var_os("SUDACHI_TRIE_BENCH_INPUTS")
        .map(|path| read_inputs(PathBuf::from(path), limit))
        .unwrap_or_else(|| synthetic_inputs(index_forms(lexicons), limit));
    let inputs = lines
        .into_iter()
        .filter(|line| !line.is_empty())
        .map(TextInput::new)
        .filter(|input| !input.starts.is_empty())
        .collect::<Vec<_>>();
    if inputs.is_empty() {
        vec![TextInput::new("東京都京都府".to_owned())]
    } else {
        inputs
    }
}

fn synthetic_inputs(forms: Vec<String>, limit: usize) -> Vec<String> {
    let mut forms = sample_inputs(forms, limit.saturating_mul(6).max(1));
    if forms.is_empty() {
        forms.push("東京都".to_owned());
        forms.push("京都府".to_owned());
    }

    let mut lines = Vec::new();
    for i in 0..limit.max(1) {
        let a = &forms[i % forms.len()];
        let b = &forms[(i * 7 + 3) % forms.len()];
        let c = &forms[(i * 13 + 5) % forms.len()];
        let d = &forms[(i * 17 + 11) % forms.len()];
        lines.push(format!("{a}{b}、{c}{d}"));
    }
    lines.extend(
        forms
            .iter()
            .take(limit / 4)
            .map(|form| format!("{form}\u{e000}")),
    );
    dedup(lines)
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

fn env_usize(name: &str) -> Option<usize> {
    env::var(name)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
}

impl TextInput {
    fn new(text: String) -> Self {
        let mut starts = Vec::new();
        let mut byte_to_start = vec![usize::MAX; text.len() + 1];
        let mut byte_boundaries = vec![false; text.len() + 1];
        for (ordinal, (offset, _)) in text.char_indices().enumerate() {
            starts.push(offset);
            byte_to_start[offset] = ordinal;
            byte_boundaries[offset] = true;
        }
        byte_boundaries[text.len()] = true;

        Self {
            text,
            starts,
            byte_to_start,
            byte_boundaries,
        }
    }

    fn bucket_for_start(&self, offset: usize) -> Option<usize> {
        self.byte_to_start
            .get(offset)
            .copied()
            .filter(|bucket| *bucket != usize::MAX)
    }

    fn is_boundary(&self, offset: usize) -> bool {
        self.byte_boundaries.get(offset).copied().unwrap_or(false)
    }
}

fn build_yada_variants(fixture: &Fixture) -> ReportResult<Vec<BuiltMatcher>> {
    let mut built = Vec::new();
    for variant in yada_variants(&fixture.profile_path) {
        let (bytes, build_time) = timed(|| compile(fixture, variant.strategy))?;
        let leaked: &'static [u8] = Box::leak(bytes.into_boxed_slice());
        let dictionary = BinaryDictionary::load_system(leaked)?;
        let trie_bytes = dictionary.lexicon.trie.total_size();
        built.push(BuiltMatcher {
            name: variant.name,
            family: "yada",
            mode: MatchMode::PrefixAllStarts,
            build_time,
            heap_bytes: trie_bytes,
            serialized_bytes: trie_bytes,
            matcher: Matcher::Yada(dictionary),
        });
    }
    Ok(built)
}

fn yada_variants(profile_path: &Path) -> Vec<YadaVariant> {
    vec![
        YadaVariant {
            name: "classic_yada",
            strategy: TrieBuildStrategy::ClassicYada,
        },
        YadaVariant {
            name: "cache_aware_uniform",
            strategy: TrieBuildStrategy::CacheAware(CacheAwareOptions {
                profile_mode: TrieProfileMode::Uniform,
                ..CacheAwareOptions::default()
            }),
        },
        YadaVariant {
            name: "cache_aware_prefix",
            strategy: TrieBuildStrategy::CacheAware(CacheAwareOptions::default()),
        },
        YadaVariant {
            name: "cache_aware_external_profile",
            strategy: TrieBuildStrategy::CacheAware(CacheAwareOptions {
                profile_mode: TrieProfileMode::ExternalKeyProfile(profile_path.to_owned()),
                ..CacheAwareOptions::default()
            }),
        },
    ]
}

fn compile(fixture: &Fixture, strategy: TrieBuildStrategy) -> ReportResult<Vec<u8>> {
    let mut builder = DictBuilder::new_system();
    builder.set_trie_build_strategy(strategy);
    builder.read_conn(fixture.matrix.as_slice())?;
    for lexicon in &fixture.lexicons {
        builder.read_lexicon(lexicon.as_slice())?;
    }
    builder.resolve()?;

    let mut bytes = Vec::new();
    builder.compile(&mut bytes)?;
    Ok(bytes)
}

fn build_alternative_matchers(
    entries: &[Entry],
) -> ReportResult<(Vec<BuiltMatcher>, Vec<FailedMatcher>)> {
    let mut built = Vec::new();
    let mut failed = Vec::new();

    let (matcher, build_time) = timed(|| {
        DoubleArrayAhoCorasick::with_values(
            entries
                .iter()
                .map(|entry| (entry.surface.as_bytes(), entry.value)),
        )
        .map_err(|error| other_error(format!("failed to build daachorse bytewise: {error:?}")))
    })?;
    let heap_bytes = matcher.heap_bytes();
    let serialized_bytes = matcher.serialize().len();
    built.push(BuiltMatcher {
        name: "daachorse_bytewise",
        family: "aho-corasick",
        mode: MatchMode::OnePassOverlapping,
        build_time,
        heap_bytes,
        serialized_bytes,
        matcher: Matcher::DaachorseBytewise(matcher),
    });

    let (matcher, build_time) = timed(|| {
        CharwiseDoubleArrayAhoCorasick::with_values(
            entries
                .iter()
                .map(|entry| (entry.surface.as_str(), entry.value)),
        )
        .map_err(|error| other_error(format!("failed to build daachorse charwise: {error:?}")))
    })?;
    let heap_bytes = matcher.heap_bytes();
    let serialized_bytes = matcher.serialize().len();
    built.push(BuiltMatcher {
        name: "daachorse_charwise",
        family: "aho-corasick",
        mode: MatchMode::OnePassOverlapping,
        build_time,
        heap_bytes,
        serialized_bytes,
        matcher: Matcher::DaachorseCharwise(matcher),
    });

    let records = entries
        .iter()
        .map(|entry| (entry.surface.as_str(), entry.value))
        .collect::<Vec<_>>();

    let (matcher, build_time) = timed(|| {
        CrawdadTrie::from_records(records.iter().copied())
            .map_err(|error| other_error(format!("failed to build crawdad trie: {error:?}")))
    })?;
    let heap_bytes = matcher.heap_bytes();
    let serialized_bytes = matcher.io_bytes();
    built.push(BuiltMatcher {
        name: "crawdad_trie",
        family: "charwise-dat",
        mode: MatchMode::PrefixAllStarts,
        build_time,
        heap_bytes,
        serialized_bytes,
        matcher: Matcher::CrawdadTrie(matcher),
    });

    match timed(|| {
        CrawdadMpTrie::from_records(records.iter().copied())
            .map_err(|error| other_error(format!("failed to build crawdad mptrie: {error:?}")))
    }) {
        Ok((matcher, build_time)) => {
            let heap_bytes = matcher.heap_bytes();
            let serialized_bytes = matcher.io_bytes();
            built.push(BuiltMatcher {
                name: "crawdad_mptrie",
                family: "charwise-dat",
                mode: MatchMode::PrefixAllStarts,
                build_time,
                heap_bytes,
                serialized_bytes,
                matcher: Matcher::CrawdadMpTrie(matcher),
            });
        }
        Err(error) => failed.push(FailedMatcher {
            name: "crawdad_mptrie",
            family: "charwise-dat",
            build_time: Duration::ZERO,
            error: error.to_string(),
        }),
    }

    let (matcher, build_time) = timed(|| {
        FstMap::from_iter(
            entries
                .iter()
                .map(|entry| (entry.surface.as_bytes(), entry.value as u64)),
        )
    })?;
    let serialized_bytes = matcher.as_fst().size();
    built.push(BuiltMatcher {
        name: "fst_map_prefix_scan",
        family: "fst",
        mode: MatchMode::PrefixAllStarts,
        build_time,
        heap_bytes: serialized_bytes,
        serialized_bytes,
        matcher: Matcher::Fst(matcher),
    });

    let (matcher, build_time) = timed(|| build_marisa(entries))?;
    let heap_bytes = matcher.total_size();
    let serialized_bytes = matcher.io_size();
    let values = entries
        .iter()
        .map(|entry| (entry.surface.as_bytes().to_vec(), entry.value))
        .collect();
    built.push(BuiltMatcher {
        name: "rsmarisa_trie",
        family: "louds",
        mode: MatchMode::PrefixAllStarts,
        build_time,
        heap_bytes,
        serialized_bytes,
        matcher: Matcher::Marisa {
            trie: matcher,
            values,
        },
    });

    Ok((built, failed))
}

fn build_marisa(entries: &[Entry]) -> ReportResult<MarisaTrie> {
    let mut keyset = MarisaKeyset::new();
    for entry in entries {
        keyset.push_back_str(&entry.surface)?;
    }
    let mut trie = MarisaTrie::new();
    trie.build(&mut keyset, 0);
    Ok(trie)
}

fn other_error(message: String) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, message)
}

fn indexed_entries(baseline: &BuiltMatcher, lexicons: &[Vec<u8>]) -> ReportResult<Vec<Entry>> {
    let mut entries = Vec::new();
    for surface in index_forms(lexicons) {
        let exact = baseline
            .matcher
            .lookup_from_start(&surface)
            .into_iter()
            .find(|entry| entry.end == surface.len());
        if let Some(entry) = exact {
            entries.push(Entry {
                surface,
                value: entry.value,
            });
        }
    }
    entries.sort_by(|left, right| left.surface.cmp(&right.surface));
    entries.dedup_by(|left, right| left.surface == right.surface);
    if entries.is_empty() {
        return Err("no indexed entries were collected from the baseline trie".into());
    }
    Ok(entries)
}

fn report_rows(built: &[BuiltMatcher], inputs: &[TextInput], rounds: usize) -> Vec<ReportRow> {
    let baseline_results = collect_all_matches(&built[0].matcher, inputs);
    let validations = built
        .iter()
        .map(|candidate| validate(&candidate.matcher, inputs, &baseline_results))
        .collect::<Vec<_>>();
    let total_starts = total_char_starts(inputs).max(1);

    let mut rows = benchmark_cases(built)
        .into_iter()
        .map(|case| {
            let validation = &validations[matcher_index(built, case.matcher)];
            let (match_ns_per_batch, checksum, storage_heap_bytes) =
                measure(&case.matcher.matcher, case.storage, inputs, rounds);
            let total_heap_bytes = case.matcher.heap_bytes + storage_heap_bytes;
            ReportRow {
                variant: case.variant_name(),
                matcher: case.matcher.name,
                family: case.matcher.family,
                mode: case.matcher.mode.name(),
                storage: case.storage.name(),
                build_ms: case.matcher.build_time.as_secs_f64() * 1000.0,
                match_ns_per_batch,
                match_ns_per_start: match_ns_per_batch as f64 / total_starts as f64,
                speed_vs_current: 1.0,
                matcher_heap_bytes: case.matcher.heap_bytes,
                storage_heap_bytes,
                total_heap_bytes,
                total_heap_vs_current: 1.0,
                serialized_bytes: case.matcher.serialized_bytes,
                same_order: validation.same_order,
                same_results: validation.same_results,
                mismatches: validation.mismatches,
                checksum,
            }
        })
        .collect::<Vec<_>>();

    let baseline_ns = rows[0].match_ns_per_batch.max(1);
    let baseline_heap = rows[0].total_heap_bytes.max(1) as f64;
    for row in &mut rows {
        row.speed_vs_current = baseline_ns as f64 / row.match_ns_per_batch.max(1) as f64;
        row.total_heap_vs_current = row.total_heap_bytes as f64 / baseline_heap;
    }
    rows
}

fn matcher_index(built: &[BuiltMatcher], matcher: &BuiltMatcher) -> usize {
    built
        .iter()
        .position(|candidate| std::ptr::eq(candidate, matcher))
        .expect("benchmark case references unknown matcher")
}

fn benchmark_cases(built: &[BuiltMatcher]) -> Vec<BenchmarkCase<'_>> {
    if !env_flag("SUDACHI_COMPARE_STORAGE_VARIANTS") {
        return built
            .iter()
            .map(|matcher| BenchmarkCase {
                matcher,
                storage: StorageKind::VecBucketsClearAll,
            })
            .collect();
    }

    let mut cases = Vec::new();
    for matcher in built {
        match matcher.name {
            "classic_yada" | "cache_aware_prefix" | "daachorse_charwise" => {
                cases.push(BenchmarkCase {
                    matcher,
                    storage: StorageKind::VecBucketsClearAll,
                });
                cases.push(BenchmarkCase {
                    matcher,
                    storage: StorageKind::FlatBucketsClearAll,
                });
                cases.push(BenchmarkCase {
                    matcher,
                    storage: StorageKind::FlatGenerationBuckets,
                });
            }
            "cache_aware_uniform" | "cache_aware_external_profile" => cases.push(BenchmarkCase {
                matcher,
                storage: StorageKind::VecBucketsClearAll,
            }),
            "daachorse_bytewise" => {
                cases.push(BenchmarkCase {
                    matcher,
                    storage: StorageKind::FlatBucketsClearAll,
                });
                cases.push(BenchmarkCase {
                    matcher,
                    storage: StorageKind::FlatGenerationBuckets,
                });
            }
            _ => cases.push(BenchmarkCase {
                matcher,
                storage: StorageKind::FlatBucketsClearAll,
            }),
        }
    }
    cases
}

fn env_flag(name: &str) -> bool {
    env::var(name)
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false)
}

impl BenchmarkCase<'_> {
    fn variant_name(&self) -> String {
        format!(
            "{}_{}+{}",
            self.matcher.name,
            self.matcher.mode.name(),
            self.storage.name()
        )
    }
}

fn collect_all_matches(matcher: &Matcher, inputs: &[TextInput]) -> Vec<Vec<PipelineMatch>> {
    let mut storage = Storage::new(StorageKind::VecBucketsClearAll);
    inputs
        .iter()
        .map(|input| {
            storage.begin(input.starts.len());
            matcher.emit(input, &mut storage);
            storage.finish_matches(&input.starts)
        })
        .collect()
}

fn validate(
    candidate: &Matcher,
    inputs: &[TextInput],
    baseline_results: &[Vec<PipelineMatch>],
) -> Validation {
    let actual_results = collect_all_matches(candidate, inputs);
    let mut same_order = true;
    let mut same_results = true;
    let mut mismatches = 0;

    for (actual, expected) in actual_results.into_iter().zip(baseline_results) {
        if &actual != expected {
            same_order = false;
        }
        if sorted(actual) != sorted(expected.clone()) {
            same_results = false;
            mismatches += 1;
        }
    }

    Validation {
        same_order,
        same_results,
        mismatches,
    }
}

impl Matcher {
    fn emit(&self, input: &TextInput, storage: &mut Storage) {
        match self {
            Matcher::Yada(dictionary) => {
                let bytes = input.text.as_bytes();
                for (bucket, &start) in input.starts.iter().enumerate() {
                    for entry in dictionary.lexicon.trie.common_prefix_iterator(bytes, start) {
                        storage.push(
                            bucket,
                            StoredMatch {
                                end: entry.end,
                                value: entry.value,
                            },
                        );
                    }
                }
            }
            Matcher::DaachorseBytewise(matcher) => {
                for entry in matcher.find_overlapping_iter(input.text.as_bytes()) {
                    if !input.is_boundary(entry.end()) {
                        continue;
                    }
                    if let Some(bucket) = input.bucket_for_start(entry.start()) {
                        storage.push(
                            bucket,
                            StoredMatch {
                                end: entry.end(),
                                value: entry.value(),
                            },
                        );
                    }
                }
            }
            Matcher::DaachorseCharwise(matcher) => {
                for entry in matcher.find_overlapping_iter(&input.text) {
                    if let Some(bucket) = input.bucket_for_start(entry.start()) {
                        storage.push(
                            bucket,
                            StoredMatch {
                                end: entry.end(),
                                value: entry.value(),
                            },
                        );
                    }
                }
            }
            Matcher::CrawdadTrie(matcher) => {
                for (bucket, &start) in input.starts.iter().enumerate() {
                    let suffix = &input.text[start..];
                    for (value, char_len) in matcher.common_prefix_search(suffix.chars()) {
                        storage.push(
                            bucket,
                            StoredMatch {
                                end: start + byte_end_for_chars(suffix, char_len),
                                value,
                            },
                        );
                    }
                }
            }
            Matcher::CrawdadMpTrie(matcher) => {
                for (bucket, &start) in input.starts.iter().enumerate() {
                    let suffix = &input.text[start..];
                    for (value, char_len) in matcher.common_prefix_search(suffix.chars()) {
                        storage.push(
                            bucket,
                            StoredMatch {
                                end: start + byte_end_for_chars(suffix, char_len),
                                value,
                            },
                        );
                    }
                }
            }
            Matcher::Fst(matcher) => {
                for (bucket, &start) in input.starts.iter().enumerate() {
                    let suffix = &input.text[start..];
                    for end in prefix_byte_ends(suffix).map(|end| start + end) {
                        if let Some(value) = matcher.get(&input.text.as_bytes()[start..end]) {
                            storage.push(
                                bucket,
                                StoredMatch {
                                    end,
                                    value: value as u32,
                                },
                            );
                        }
                    }
                }
            }
            Matcher::Marisa { trie, values } => {
                for (bucket, &start) in input.starts.iter().enumerate() {
                    let suffix = &input.text[start..];
                    let mut agent = MarisaAgent::new();
                    agent.set_query_str(suffix);
                    while trie.common_prefix_search(&mut agent) {
                        let key = agent.key().as_bytes();
                        if let Some(value) = values.get(key) {
                            storage.push(
                                bucket,
                                StoredMatch {
                                    end: start + key.len(),
                                    value: *value,
                                },
                            );
                        }
                    }
                }
            }
        }
    }

    fn lookup_from_start(&self, input: &str) -> Vec<PipelineMatch> {
        let text = TextInput::new(input.to_owned());
        let mut storage = Storage::new(StorageKind::VecBucketsClearAll);
        storage.begin(text.starts.len().min(1));
        match self {
            Matcher::Yada(dictionary) => {
                for entry in dictionary
                    .lexicon
                    .trie
                    .common_prefix_iterator(input.as_bytes(), 0)
                {
                    storage.push(
                        0,
                        StoredMatch {
                            end: entry.end,
                            value: entry.value,
                        },
                    );
                }
            }
            _ => self.emit(&text, &mut storage),
        }
        storage.finish_matches(&[0])
    }
}

fn measure(
    matcher: &Matcher,
    storage_kind: StorageKind,
    inputs: &[TextInput],
    rounds: usize,
) -> (u128, u64, usize) {
    let mut storage = Storage::new(storage_kind);
    let started = Instant::now();
    let mut checksum = 0u64;
    for _ in 0..rounds {
        for input in inputs {
            storage.begin(input.starts.len());
            matcher.emit(input, &mut storage);
            checksum = storage.finish_checksum(&input.starts, checksum);
        }
    }
    black_box(checksum);
    (
        started.elapsed().as_nanos() / rounds.max(1) as u128,
        checksum,
        storage.retained_bytes(),
    )
}

enum Storage {
    VecBuckets(VecBucketStorage),
    FlatBuckets(FlatBucketStorage),
    FlatGeneration(FlatGenerationStorage),
}

impl Storage {
    fn new(kind: StorageKind) -> Self {
        match kind {
            StorageKind::VecBucketsClearAll => Self::VecBuckets(VecBucketStorage::default()),
            StorageKind::FlatBucketsClearAll => Self::FlatBuckets(FlatBucketStorage::default()),
            StorageKind::FlatGenerationBuckets => {
                Self::FlatGeneration(FlatGenerationStorage::default())
            }
        }
    }

    fn begin(&mut self, bucket_count: usize) {
        match self {
            Self::VecBuckets(storage) => storage.begin(bucket_count),
            Self::FlatBuckets(storage) => storage.begin(bucket_count),
            Self::FlatGeneration(storage) => storage.begin(bucket_count),
        }
    }

    fn push(&mut self, bucket: usize, item: StoredMatch) {
        match self {
            Self::VecBuckets(storage) => storage.push(bucket, item),
            Self::FlatBuckets(storage) => storage.push(bucket, item),
            Self::FlatGeneration(storage) => storage.push(bucket, item),
        }
    }

    fn finish_checksum(&mut self, starts: &[usize], checksum: u64) -> u64 {
        match self {
            Self::VecBuckets(storage) => storage.finish_checksum(starts, checksum),
            Self::FlatBuckets(storage) => storage.finish_checksum(starts, checksum),
            Self::FlatGeneration(storage) => storage.finish_checksum(starts, checksum),
        }
    }

    fn finish_matches(&mut self, starts: &[usize]) -> Vec<PipelineMatch> {
        match self {
            Self::VecBuckets(storage) => storage.finish_matches(starts),
            Self::FlatBuckets(storage) => storage.finish_matches(starts),
            Self::FlatGeneration(storage) => storage.finish_matches(starts),
        }
    }

    fn retained_bytes(&self) -> usize {
        match self {
            Self::VecBuckets(storage) => storage.retained_bytes(),
            Self::FlatBuckets(storage) => storage.retained_bytes(),
            Self::FlatGeneration(storage) => storage.retained_bytes(),
        }
    }
}

#[derive(Default)]
struct VecBucketStorage {
    buckets: Vec<Vec<StoredMatch>>,
}

impl VecBucketStorage {
    fn begin(&mut self, bucket_count: usize) {
        if self.buckets.len() < bucket_count {
            self.buckets.resize_with(bucket_count, Vec::new);
        }
        for bucket in self.buckets.iter_mut().take(bucket_count) {
            bucket.clear();
        }
    }

    fn push(&mut self, bucket: usize, item: StoredMatch) {
        if let Some(items) = self.buckets.get_mut(bucket) {
            items.push(item);
        }
    }

    fn finish_checksum(&self, starts: &[usize], mut checksum: u64) -> u64 {
        for (bucket, &start) in starts.iter().enumerate() {
            if let Some(items) = self.buckets.get(bucket) {
                for item in items {
                    checksum = mix_checksum(checksum, start, *item);
                }
            }
        }
        checksum
    }

    fn finish_matches(&self, starts: &[usize]) -> Vec<PipelineMatch> {
        let mut matches = Vec::new();
        for (bucket, &start) in starts.iter().enumerate() {
            if let Some(items) = self.buckets.get(bucket) {
                matches.extend(items.iter().map(|item| PipelineMatch {
                    start,
                    end: item.end,
                    value: item.value,
                }));
            }
        }
        matches
    }

    fn retained_bytes(&self) -> usize {
        self.buckets.capacity() * size_of::<Vec<StoredMatch>>()
            + self
                .buckets
                .iter()
                .map(|bucket| bucket.capacity() * size_of::<StoredMatch>())
                .sum::<usize>()
    }
}

#[derive(Default)]
struct FlatBucketStorage {
    heads: Vec<usize>,
    tails: Vec<usize>,
    next: Vec<usize>,
    items: Vec<StoredMatch>,
    bucket_count: usize,
}

impl FlatBucketStorage {
    fn begin(&mut self, bucket_count: usize) {
        self.bucket_count = bucket_count;
        if self.heads.len() < bucket_count {
            self.heads.resize(bucket_count, usize::MAX);
            self.tails.resize(bucket_count, usize::MAX);
        }
        for bucket in 0..bucket_count {
            self.heads[bucket] = usize::MAX;
            self.tails[bucket] = usize::MAX;
        }
        self.items.clear();
        self.next.clear();
    }

    fn push(&mut self, bucket: usize, item: StoredMatch) {
        if bucket < self.bucket_count {
            let index = self.items.len();
            self.items.push(item);
            self.next.push(usize::MAX);
            if self.heads[bucket] == usize::MAX {
                self.heads[bucket] = index;
            } else {
                self.next[self.tails[bucket]] = index;
            }
            self.tails[bucket] = index;
        }
    }

    fn finish_checksum(&mut self, starts: &[usize], checksum: u64) -> u64 {
        checksum_from_linked(starts, &self.heads, &self.next, &self.items, checksum)
    }

    fn finish_matches(&mut self, starts: &[usize]) -> Vec<PipelineMatch> {
        matches_from_linked(starts, &self.heads, &self.next, &self.items)
    }

    fn retained_bytes(&self) -> usize {
        self.heads.capacity() * size_of::<usize>()
            + self.tails.capacity() * size_of::<usize>()
            + self.next.capacity() * size_of::<usize>()
            + self.items.capacity() * size_of::<StoredMatch>()
    }
}

#[derive(Default)]
struct FlatGenerationStorage {
    heads: Vec<usize>,
    tails: Vec<usize>,
    generations: Vec<u32>,
    next: Vec<usize>,
    items: Vec<StoredMatch>,
    bucket_count: usize,
    current_generation: u32,
}

impl FlatGenerationStorage {
    fn begin(&mut self, bucket_count: usize) {
        self.bucket_count = bucket_count;
        self.current_generation = self.current_generation.wrapping_add(1).max(1);
        if self.heads.len() < bucket_count {
            self.heads.resize(bucket_count, usize::MAX);
            self.tails.resize(bucket_count, usize::MAX);
            self.generations.resize(bucket_count, 0);
        }
        if self.current_generation == u32::MAX {
            self.generations.fill(0);
            self.current_generation = 1;
        }
        self.items.clear();
        self.next.clear();
    }

    fn push(&mut self, bucket: usize, item: StoredMatch) {
        if bucket >= self.bucket_count {
            return;
        }
        if self.generations[bucket] != self.current_generation {
            self.generations[bucket] = self.current_generation;
            self.heads[bucket] = usize::MAX;
            self.tails[bucket] = usize::MAX;
        }
        let index = self.items.len();
        self.items.push(item);
        self.next.push(usize::MAX);
        if self.heads[bucket] == usize::MAX {
            self.heads[bucket] = index;
        } else {
            self.next[self.tails[bucket]] = index;
        }
        self.tails[bucket] = index;
    }

    fn finish_checksum(&mut self, starts: &[usize], checksum: u64) -> u64 {
        let current_generation = self.current_generation;
        checksum_from_linked_with_generation(
            starts,
            &self.heads,
            &self.next,
            &self.items,
            &self.generations,
            current_generation,
            checksum,
        )
    }

    fn finish_matches(&mut self, starts: &[usize]) -> Vec<PipelineMatch> {
        matches_from_linked_with_generation(
            starts,
            &self.heads,
            &self.next,
            &self.items,
            &self.generations,
            self.current_generation,
        )
    }

    fn retained_bytes(&self) -> usize {
        self.heads.capacity() * size_of::<usize>()
            + self.tails.capacity() * size_of::<usize>()
            + self.generations.capacity() * size_of::<u32>()
            + self.next.capacity() * size_of::<usize>()
            + self.items.capacity() * size_of::<StoredMatch>()
    }
}

fn checksum_from_linked(
    starts: &[usize],
    heads: &[usize],
    next: &[usize],
    items: &[StoredMatch],
    mut checksum: u64,
) -> u64 {
    for (bucket, &start) in starts.iter().enumerate() {
        let mut index = heads[bucket];
        while index != usize::MAX {
            checksum = mix_checksum(checksum, start, items[index]);
            index = next[index];
        }
    }
    checksum
}

fn matches_from_linked(
    starts: &[usize],
    heads: &[usize],
    next: &[usize],
    items: &[StoredMatch],
) -> Vec<PipelineMatch> {
    let mut matches = Vec::new();
    for (bucket, &start) in starts.iter().enumerate() {
        let mut index = heads[bucket];
        while index != usize::MAX {
            let item = items[index];
            matches.push(PipelineMatch {
                start,
                end: item.end,
                value: item.value,
            });
            index = next[index];
        }
    }
    matches
}

fn checksum_from_linked_with_generation(
    starts: &[usize],
    heads: &[usize],
    next: &[usize],
    items: &[StoredMatch],
    generations: &[u32],
    current_generation: u32,
    mut checksum: u64,
) -> u64 {
    for (bucket, &start) in starts.iter().enumerate() {
        if generations[bucket] != current_generation {
            continue;
        }
        let mut index = heads[bucket];
        while index != usize::MAX {
            checksum = mix_checksum(checksum, start, items[index]);
            index = next[index];
        }
    }
    checksum
}

fn matches_from_linked_with_generation(
    starts: &[usize],
    heads: &[usize],
    next: &[usize],
    items: &[StoredMatch],
    generations: &[u32],
    current_generation: u32,
) -> Vec<PipelineMatch> {
    let mut matches = Vec::new();
    for (bucket, &start) in starts.iter().enumerate() {
        if generations[bucket] != current_generation {
            continue;
        }
        let mut index = heads[bucket];
        while index != usize::MAX {
            let item = items[index];
            matches.push(PipelineMatch {
                start,
                end: item.end,
                value: item.value,
            });
            index = next[index];
        }
    }
    matches
}

fn mix_checksum(checksum: u64, start: usize, item: StoredMatch) -> u64 {
    checksum
        .wrapping_mul(16_777_619)
        .wrapping_add(start as u64)
        .wrapping_mul(16_777_619)
        .wrapping_add(item.end as u64)
        .wrapping_mul(16_777_619)
        .wrapping_add(item.value as u64)
}

impl MatchMode {
    fn name(self) -> &'static str {
        match self {
            Self::PrefixAllStarts => "prefix_all_starts",
            Self::OnePassOverlapping => "one_pass_overlapping",
        }
    }
}

impl StorageKind {
    fn name(self) -> &'static str {
        match self {
            Self::VecBucketsClearAll => "vec_buckets_clear_all",
            Self::FlatBucketsClearAll => "flat_buckets_clear_all",
            Self::FlatGenerationBuckets => "flat_generation_buckets",
        }
    }
}

fn index_forms(lexicons: &[Vec<u8>]) -> Vec<String> {
    let mut forms = Vec::new();
    for lexicon in lexicons {
        forms.extend(index_forms_with_headers(lexicon));
    }
    if forms.is_empty() {
        for lexicon in lexicons {
            forms.extend(index_forms_without_headers(lexicon));
        }
    }
    dedup(forms)
}

fn index_forms_with_headers(lexicon: &[u8]) -> Vec<String> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(lexicon);
    let index_col = reader
        .headers()
        .ok()
        .and_then(|headers| {
            headers
                .iter()
                .position(|header| header.eq_ignore_ascii_case("index_form"))
        })
        .unwrap_or(0);
    reader
        .records()
        .flatten()
        .filter_map(|record| record.get(index_col).map(ToOwned::to_owned))
        .filter(|form| !form.is_empty())
        .collect()
}

fn index_forms_without_headers(lexicon: &[u8]) -> Vec<String> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(lexicon);
    reader
        .records()
        .flatten()
        .filter_map(|record| record.get(0).map(ToOwned::to_owned))
        .filter(|form| {
            !form.is_empty()
                && !form.eq_ignore_ascii_case("index_form")
                && !form.eq_ignore_ascii_case("indexform")
        })
        .collect()
}

fn sample_inputs(mut inputs: Vec<String>, limit: usize) -> Vec<String> {
    inputs.sort();
    inputs.dedup();
    if limit == 0 || inputs.len() <= limit {
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

fn external_profile_path(inputs: &[TextInput]) -> (PathBuf, Option<NamedTempFile>) {
    if let Some(path) = env::var_os("SUDACHI_TRIE_BENCH_PROFILE") {
        return (PathBuf::from(path), None);
    }

    let mut file = NamedTempFile::new().expect("failed to create benchmark profile");
    let max = inputs.len();
    for (rank, input) in inputs.iter().enumerate() {
        write_profile_key(&mut file, input.text.as_bytes());
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

fn prefix_byte_ends(input: &str) -> impl Iterator<Item = usize> + '_ {
    input
        .char_indices()
        .map(|(idx, ch)| idx + ch.len_utf8())
        .filter(|end| *end <= input.len())
}

fn byte_end_for_chars(input: &str, char_len: usize) -> usize {
    input
        .char_indices()
        .nth(char_len)
        .map(|(idx, _)| idx)
        .unwrap_or_else(|| input.len())
}

fn sorted(mut entries: Vec<PipelineMatch>) -> Vec<PipelineMatch> {
    entries.sort();
    entries
}

fn timed<T, E>(f: impl FnOnce() -> Result<T, E>) -> Result<(T, Duration), E> {
    let started = Instant::now();
    let value = f()?;
    Ok((value, started.elapsed()))
}

fn total_char_starts(inputs: &[TextInput]) -> usize {
    inputs.iter().map(|input| input.starts.len()).sum()
}

fn print_summary(fixture: &Fixture) {
    let chars: usize = fixture
        .inputs
        .iter()
        .map(|input| input.text.chars().count())
        .sum();
    println!("# data_set: {}", fixture.data_set);
    println!("# inputs: {}", fixture.inputs.len());
    println!("# chars_per_batch: {}", chars);
    println!(
        "# char_starts_per_batch: {}",
        total_char_starts(&fixture.inputs)
    );
    println!("# rounds: {}", fixture.rounds);
}

fn print_tsv(rows: &[ReportRow], failed: &[FailedMatcher]) {
    println!(
        "variant\tmatcher\tfamily\tmode\tstorage\tbuild_ms\tmatch_ns_per_batch\tmatch_ns_per_start\tspeed_vs_current\tmatcher_heap_bytes\tstorage_heap_bytes\ttotal_heap_bytes\ttotal_heap_vs_current\tserialized_bytes\tsame_order\tsame_results\tmismatches\tchecksum"
    );
    for row in rows {
        println!(
            "{}\t{}\t{}\t{}\t{}\t{:.3}\t{}\t{:.3}\t{:.3}\t{}\t{}\t{}\t{:.3}\t{}\t{}\t{}\t{}\t{}",
            row.variant,
            row.matcher,
            row.family,
            row.mode,
            row.storage,
            row.build_ms,
            row.match_ns_per_batch,
            row.match_ns_per_start,
            row.speed_vs_current,
            row.matcher_heap_bytes,
            row.storage_heap_bytes,
            row.total_heap_bytes,
            row.total_heap_vs_current,
            row.serialized_bytes,
            row.same_order,
            row.same_results,
            row.mismatches,
            row.checksum
        );
    }
    for row in failed {
        println!(
            "{}\t{}\t{}\tn/a\tn/a\t{:.3}\tn/a\tn/a\tn/a\tn/a\tn/a\tn/a\tn/a\tn/a\tfalse\tfalse\tn/a\t{}",
            row.name,
            row.name,
            row.family,
            row.build_time.as_secs_f64() * 1000.0,
            row.error
        );
    }
}

fn print_markdown(rows: &[ReportRow], failed: &[FailedMatcher]) {
    println!("| Variant | Mode | Storage | Build ms | Match ns/batch | ns/start | Speed vs current | Matcher heap | Storage heap | Total heap | Total heap vs current | Serialized bytes | Same order | Same results | Mismatches | Note |");
    println!("|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|---|---:|---|");
    for row in rows {
        println!(
            "| `{}` | {} | {} | {:.3} | {} | {:.2} | {:.2}x | {} | {} | {} | {:.2}x | {} | {} | {} | {} |  |",
            row.variant,
            row.mode,
            row.storage,
            row.build_ms,
            row.match_ns_per_batch,
            row.match_ns_per_start,
            row.speed_vs_current,
            row.matcher_heap_bytes,
            row.storage_heap_bytes,
            row.total_heap_bytes,
            row.total_heap_vs_current,
            row.serialized_bytes,
            yes_no(row.same_order),
            yes_no(row.same_results),
            row.mismatches
        );
    }
    for row in failed {
        println!(
            "| `{}` | n/a | n/a | {:.3} | n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a | no | no | n/a | {} |",
            row.name,
            row.build_time.as_secs_f64() * 1000.0,
            row.error.replace('|', "\\|")
        );
    }
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}
