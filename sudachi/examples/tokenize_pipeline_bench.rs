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

//! End-to-end tokenization throughput: scalar vs pipelined+prefetched lattice
//! lookup (issue #117). Both paths must produce the same morpheme count; only
//! the dictionary-lookup driver differs.
//!
//! ```bash
//! SUDACHI_BENCH_CONFIG=resources/sudachi.json \
//! SUDACHI_BENCH_INPUTS=target/issue-117-corpora/kyoto-leads.txt \
//! cargo run -p sudachi --release --example tokenize_pipeline_bench
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

fn run_pass(
    tok: &mut StatefulTokenizer<Arc<JapaneseDictionary>>,
    result: &mut MorphemeList<Arc<JapaneseDictionary>>,
    lines: &[&str],
) -> usize {
    let mut total = 0;
    for line in lines {
        tok.reset().push_str(line);
        tok.do_tokenize().expect("tokenization failed");
        result.collect_results(tok).expect("collect failed");
        total += result.len();
    }
    total
}

fn main() {
    let config_path = env_path("SUDACHI_BENCH_CONFIG", "resources/sudachi.json");
    let resource_dir = config_path
        .parent()
        .map(|p| p.to_path_buf())
        .filter(|p| !p.as_os_str().is_empty());
    // Optional system-dictionary override (the committed resources/system.dic is
    // an older V0-format dict; point this at a freshly built current-format one).
    let dict_override = std::env::var_os("SUDACHI_BENCH_DICT").map(PathBuf::from);
    let config = Config::new(Some(config_path.clone()), resource_dir, dict_override)
        .expect("failed to load config");
    let dict = Arc::new(JapaneseDictionary::from_cfg(&config).expect("failed to load dictionary"));

    let inputs_path = env_path(
        "SUDACHI_BENCH_INPUTS",
        "target/issue-117-corpora/kyoto-leads.txt",
    );
    let limit = env_usize("SUDACHI_BENCH_LIMIT", usize::MAX);
    let trials = env_usize("SUDACHI_BENCH_TRIALS", 7);
    let text = std::fs::read_to_string(&inputs_path).expect("failed to read inputs");
    let lines: Vec<&str> = text.lines().take(limit).collect();
    let total_chars: usize = lines.iter().map(|l| l.chars().count()).sum();

    let mut tok = StatefulTokenizer::new(dict.clone(), Mode::C);
    let mut result = MorphemeList::empty(dict.clone());

    // Warm up and check both paths agree on output volume.
    tok.set_pipelined_lookup(false);
    let morphs_scalar = run_pass(&mut tok, &mut result, &lines);
    tok.set_pipelined_lookup(true);
    let morphs_pipelined = run_pass(&mut tok, &mut result, &lines);
    assert_eq!(
        morphs_scalar, morphs_pipelined,
        "scalar and pipelined disagree on total morpheme count"
    );

    let mut best_scalar = Duration::MAX;
    let mut best_pipelined = Duration::MAX;
    for t in 0..trials {
        // Alternate which path runs first so neither systematically benefits
        // from the other warming the caches.
        let order = if t % 2 == 0 {
            [false, true]
        } else {
            [true, false]
        };
        for &pipelined in &order {
            tok.set_pipelined_lookup(pipelined);
            let start = Instant::now();
            run_pass(&mut tok, &mut result, &lines);
            let elapsed = start.elapsed();
            if pipelined {
                best_pipelined = best_pipelined.min(elapsed);
            } else {
                best_scalar = best_scalar.min(elapsed);
            }
        }
    }

    let n = lines.len().max(1);
    let report = |name: &str, d: Duration| {
        let ns = d.as_nanos() as f64;
        println!(
            "{name:<20} {:>9.2} ms  {:>8.0} ns/sentence  {:>6.2} ns/char  {:>9.0} sent/s",
            ns / 1e6,
            ns / n as f64,
            ns / total_chars.max(1) as f64,
            n as f64 / d.as_secs_f64(),
        );
    };

    println!("# config: {}", config_path.display());
    println!("# inputs: {}", inputs_path.display());
    println!(
        "# sentences: {n}, chars: {total_chars}, morphemes: {morphs_scalar}, trials: {trials} (best-of)"
    );
    report("scalar", best_scalar);
    report("pipelined+prefetch", best_pipelined);
    println!(
        "# speedup (scalar / pipelined): {:.4}x",
        best_scalar.as_secs_f64() / best_pipelined.as_secs_f64()
    );
}
