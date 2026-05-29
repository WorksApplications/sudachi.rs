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
use std::time::Instant;

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

    let mut scalar_ms: Vec<f64> = Vec::new();
    let mut pipelined_ms: Vec<f64> = Vec::new();
    for t in 0..trials {
        // Alternate which path runs first so neither systematically benefits
        // from the other warming the caches.
        let order = if t % 2 == 0 {
            [false, true]
        } else {
            [true, false]
        };
        for &pipelined in &order {
            // SUDACHI_BENCH_ONLY=scalar|pipelined restricts to one path (for profiling).
            match std::env::var("SUDACHI_BENCH_ONLY").ok().as_deref() {
                Some("scalar") if pipelined => continue,
                Some("pipelined") if !pipelined => continue,
                _ => {}
            }
            tok.set_pipelined_lookup(pipelined);
            let start = Instant::now();
            run_pass(&mut tok, &mut result, &lines);
            let ms = start.elapsed().as_secs_f64() * 1e3;
            if pipelined {
                pipelined_ms.push(ms);
            } else {
                scalar_ms.push(ms);
            }
        }
    }

    // (min, median, mean, coefficient-of-variation %)
    fn stats(samples: &mut [f64]) -> (f64, f64, f64, f64) {
        samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let min = samples[0];
        let median = samples[samples.len() / 2];
        let mean = samples.iter().sum::<f64>() / samples.len() as f64;
        let var = samples.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / samples.len() as f64;
        let cv = if mean > 0.0 {
            var.sqrt() / mean * 100.0
        } else {
            0.0
        };
        (min, median, mean, cv)
    }

    let n = lines.len().max(1);
    let report = |name: &str, samples: &mut Vec<f64>| -> (f64, f64) {
        if samples.is_empty() {
            return (0.0, 0.0);
        }
        let (min, median, mean, cv) = stats(samples);
        println!(
            "{name:<20} min {:>7.2}  median {:>7.2}  mean {:>7.2} ms  cv {:>4.1}%  | {:>8.0} sent/s  {:>6.2} ns/char",
            min,
            median,
            mean,
            cv,
            n as f64 / (median / 1e3),
            median * 1e6 / total_chars.max(1) as f64,
        );
        (min, median)
    };

    println!("# config: {}", config_path.display());
    println!("# inputs: {}", inputs_path.display());
    println!(
        "# sentences: {n}, chars: {total_chars}, morphemes: {morphs_scalar}, trials: {trials}"
    );
    let (smin, smed) = report("scalar", &mut scalar_ms);
    let (pmin, pmed) = report("pipelined+prefetch", &mut pipelined_ms);
    if smed > 0.0 && pmed > 0.0 {
        println!(
            "# speedup  median {:.4}x   best-of {:.4}x",
            smed / pmed,
            smin / pmin
        );
    }
}
