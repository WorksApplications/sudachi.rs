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

//! Batch tokenization throughput vs thread count. Sentences are independent and
//! the dictionary is shared read-only via `Arc`, so throughput scales close to
//! linearly with cores. Each worker owns its own `StatefulTokenizer`.
//!
//! ```bash
//! SUDACHI_BENCH_DICT=/path/system.dic \
//! cargo run -p sudachi --release --example tokenize_parallel_bench
//! ```

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use sudachi::analysis::mlist::MorphemeList;
use sudachi::analysis::stateful_tokenizer::StatefulTokenizer;
use sudachi::analysis::Mode;
use sudachi::config::Config;
use sudachi::dic::dictionary::JapaneseDictionary;

fn env_path(key: &str, default: &str) -> PathBuf {
    std::env::var_os(key)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(default))
}

fn env_usize(key: &str, default: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// Tokenize a slice of sentences with one fresh tokenizer; returns morpheme count.
fn tokenize_chunk(dict: Arc<JapaneseDictionary>, lines: &[String]) -> usize {
    let mut tok = StatefulTokenizer::new(dict.clone(), Mode::C);
    let mut result = MorphemeList::empty(dict);
    let mut total = 0;
    for line in lines {
        tok.reset().push_str(line);
        tok.do_tokenize().expect("tokenization failed");
        result.collect_results(&mut tok).expect("collect failed");
        total += result.len();
    }
    total
}

fn run_threads(
    dict: &Arc<JapaneseDictionary>,
    lines: &[String],
    threads: usize,
) -> (Duration, usize) {
    let chunk = lines.len().div_ceil(threads.max(1));
    let start = Instant::now();
    let total: usize = std::thread::scope(|scope| {
        let handles: Vec<_> = lines
            .chunks(chunk.max(1))
            .map(|part| {
                let dict = Arc::clone(dict);
                scope.spawn(move || tokenize_chunk(dict, part))
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).sum()
    });
    (start.elapsed(), total)
}

fn main() {
    let config_path = env_path("SUDACHI_BENCH_CONFIG", "resources/sudachi.json");
    let resource_dir = config_path
        .parent()
        .map(|p| p.to_path_buf())
        .filter(|p| !p.as_os_str().is_empty());
    let dict_override = std::env::var_os("SUDACHI_BENCH_DICT").map(PathBuf::from);
    let config =
        Config::new(Some(config_path), resource_dir, dict_override).expect("failed to load config");
    let dict = Arc::new(JapaneseDictionary::from_cfg(&config).expect("failed to load dictionary"));

    let inputs_path = env_path(
        "SUDACHI_BENCH_INPUTS",
        "target/issue-117-corpora/kyoto-leads.txt",
    );
    let limit = env_usize("SUDACHI_BENCH_LIMIT", usize::MAX);
    let trials = env_usize("SUDACHI_BENCH_TRIALS", 5);
    let text = std::fs::read_to_string(&inputs_path).expect("failed to read inputs");
    let lines: Vec<String> = text.lines().take(limit).map(|s| s.to_owned()).collect();

    let max_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(8);
    let mut thread_counts = vec![1usize, 2, 4, 8];
    thread_counts.retain(|&t| t < max_threads);
    thread_counts.push(max_threads);

    // Warm up / correctness: morpheme count must not depend on thread count.
    let (_, base_total) = run_threads(&dict, &lines, 1);

    println!("# inputs: {}", inputs_path.display());
    println!(
        "# sentences: {}, morphemes: {base_total}, max_threads: {max_threads}, trials: {trials} (best-of)",
        lines.len()
    );

    let mut single = Duration::MAX;
    for &threads in &thread_counts {
        let mut best = Duration::MAX;
        for _ in 0..trials {
            let (dt, total) = run_threads(&dict, &lines, threads);
            assert_eq!(
                total, base_total,
                "morpheme count changed at {threads} threads"
            );
            best = best.min(dt);
        }
        if threads == 1 {
            single = best;
        }
        let sent_s = lines.len() as f64 / best.as_secs_f64();
        let speedup = single.as_secs_f64() / best.as_secs_f64();
        println!(
            "threads={threads:<3} {:>8.2} ms  {:>10.0} sent/s  {:>5.2}x vs 1 thread",
            best.as_nanos() as f64 / 1e6,
            sent_s,
            speedup,
        );
    }
}
