# Runtime prefetch-friendly trie lookup (issue #117)

Issue #117 asks whether Sudachi can keep its `darts-clone`/Yada-compatible
double-array trie but make lookups load the next symbol's nodes into L1
"speculatively, even with not 100% accuracy". This document describes the
**runtime** answer: a software-pipelined, prefetch-assisted common-prefix
search. It is the default in lattice construction.

It does **not** change the serialized dictionary at all: the trie bytes, the
runtime `u32` double-array layout, word-id table, and build pipeline are
untouched. Build time is unaffected. Only *how* the runtime walks the array
changes.

See `docs/issue-117-performance-report.md` for the separate, earlier
exploration of a *build-time* node relayout (`--trie-layout cache-aware`), which
was rejected: it costs ~14x build time for ≤1.10x lookup and is opt-in only.

## The idea

A double-array common-prefix walk is a pointer chase:

```text
node_pos ^= byte;  unit = trie[node_pos];  node_pos ^= offset(unit);  // repeat
```

Each step depends on the previous load, so a single walk cannot overlap its own
memory latency. But Sudachi runs one **independent** walk per character boundary
of a sentence (`build_lattice`), and a walk's *next* load address —
`trie[node_pos ^ next_byte]` — is known one step ahead without another load.

So we keep `K` independent walks in flight, advance them round-robin one byte at
a time, and `prefetch` each lane's next node while the other lanes run. This is
classic software-pipelined prefetching / group prefetching (Chen & Ailamaki;
Kocberber et al. "AMAC"). A mis-speculated lane (one that hits a dead end before
its prefetched line is used) just wastes a harmless hint — exactly the "not 100%
accuracy" the issue allows.

The shared per-step arithmetic (`step_once` in
`sudachi/src/dic/lexicon/trie.rs`) is identical to the scalar iterator, so the
two paths are byte-for-byte equivalent by construction.

## API

- `Trie::common_prefix_batch(input, starts, emit)` — pipelined search from many
  start offsets; `emit(bucket, value, end)` per match, with per-`bucket` order
  identical to `common_prefix_iterator(input, starts[bucket])`.
- `Lexicon::lookup_batch` / `LexiconSet::lookup_batch` — the same over the
  dictionary set, preserving `lookup()`'s ordering (user dicts first, then
  system, each in trie-walk order).
- `StatefulTokenizer::set_pipelined_lookup(bool)` — override the default
  (on) for benchmarking; the result is unchanged either way.

`K = 4` lanes (`Trie::DEFAULT_PREFETCH_LANES`) is the measured sweet spot: enough
memory-level parallelism to hide L2/L3 misses, while the lane state stays in
registers. Larger `K` adds scheduler overhead that outweighs the extra latency
hiding on this workload.

Prefetch primitive (`sudachi/src/util/prefetch.rs`): `_mm_prefetch` on
x86/x86_64, `prfm pldl1keep` on aarch64, no-op elsewhere — all stable Rust.

## Correctness

- `dic::lexicon::trie::tests` — `common_prefix_batch` matches
  `common_prefix_iterator` across lane counts {1,2,3,8,16}, prefetch on/off,
  including empty input, misses, multibyte, leaf-with-children, and
  sparse/unordered/duplicate starts.
- `tests/pipelined_lookup_parity.rs` — full tokenization (modes A/B/C) is
  identical between the scalar and pipelined lattice builders.
- The entire existing test suite runs with the pipelined path as the default
  and is unchanged.

## Results

Hardware: Apple M4 Max (aarch64), rustc 1.95.0, `--release`. Real SudachiDict
tier lexicons + the full `matrix.def`; corpus = 16,051 Kyoto lead sentences
(`target/issue-117-corpora/kyoto-leads.txt`). Connection matrix (~71 MB) is the
same across tiers, so even the "small" dictionary tokenization is cache-pressured.

### End-to-end tokenization (what users run) — best-of-15

| Dictionary | trie size | scalar (sent/s) | pipelined (sent/s) | speedup |
|---|---:|---:|---:|---:|
| small | 8.4 MB | 84,943 | 91,877 | **1.082x** |
| core | 27.5 MB | 82,573 | 91,742 | **1.111x** |
| full | 72.2 MB | 78,585 | 88,322 | **1.124x** |

~8–12% faster whole-pipeline tokenization, identical morpheme output, zero build
or memory cost. Laptop run-to-run variance is real; the win is consistently
positive across runs.

### Isolated common-prefix lookup (warm-cache micro-benchmark) — ns/start

| Dictionary | classic | pipelined+prefetch (K=4) | speedup |
|---|---:|---:|---:|
| small | 24.05 | 25.33 | 0.95x |
| core | 29.50 | 27.58 | 1.07x |
| full | 32.44 | 30.16 | 1.08x |

The isolated benchmark repeats lookups over the same inputs, so the trie stays
warm in cache and there is little latency to hide on smaller tiers — the
scheduler overhead even shows a small regression on `small`. This understates
the real workload: in `build_lattice` the trie competes with the connection
matrix and lattice for cache, so misses are more frequent and prefetch helps
more — hence the uniformly larger end-to-end wins above. Pure software
pipelining without the prefetch hint is slower than scalar on every tier, so the
explicit prefetch is doing the work, not just the reordering.

## Decision

Pipelined + prefetched lookup is the **default** in `build_lattice`:
format-preserving, zero build/size/memory cost, byte-for-byte identical output,
and ~8–12% faster end-to-end on real dictionaries. `set_pipelined_lookup(false)`
restores the scalar path. On targets without a prefetch primitive the hint is a
no-op and the (arch-neutral) pipelining still applies.

## Reproduce

End-to-end (no extra dependencies):

```bash
cargo run -p sudachi-cli --release -- build -o /tmp/full.dic \
  -m target/bench-lookup/raw/unzipped/matrix/matrix.def \
  target/bench-lookup/raw/unzipped/small/small_lex.csv \
  target/bench-lookup/raw/unzipped/core/core_lex.csv \
  target/bench-lookup/raw/unzipped/notcore/notcore_lex.csv

SUDACHI_BENCH_DICT=/tmp/full.dic SUDACHI_BENCH_TRIALS=15 \
  cargo run -p sudachi --release --example tokenize_pipeline_bench
```

Isolated lookup sweep (lane counts, prefetch on/off) needs the optional
comparison crates:

```bash
SUDACHI_SKIP_BUILD_TIME_VARIANTS=1 \
SUDACHI_TRIE_BENCH_MATRIX=target/bench-lookup/raw/unzipped/matrix/matrix.def \
SUDACHI_TRIE_BENCH_INPUTS=target/issue-117-corpora/kyoto-leads.txt \
SUDACHI_TRIE_BENCH_DATA_SET_NAME=full \
SUDACHI_TRIE_BENCH_LEXICONS=.../small_lex.csv:.../core_lex.csv:.../notcore_lex.csv \
cargo run -p sudachi --release --features matcher-comparison \
  --example dictionary_matcher_report
```
