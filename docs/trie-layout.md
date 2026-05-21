# Cache-aware trie layout

Issue #117 is implemented as a build-time layout strategy. The serialized trie
remains the existing Yada/Darts-compatible `u32` double-array representation:
runtime lookup, word-id table offsets, and dictionary block layout are unchanged.

The default builder is still `ClassicYada`, so existing dictionary builds keep the
same bytes unless the new strategy is selected explicitly.

```bash
cargo run -p sudachi-cli -- build \
  --trie-layout cache-aware \
  --trie-candidate-window 16 \
  --trie-cache-line-bytes 64 \
  -m resources/matrix.def \
  -o system.dic \
  path/to/lex.csv
```

An optional external profile can be supplied with `--trie-profile`. The format is
one entry per line:

```text
# surface<TAB>count
東京	120000
hex:E4BAACE983BD	80000
```

`hex:` keys are interpreted as raw bytes. Missing profile entries use weight `1`.

The cache-aware builder keeps deterministic output and uses three heuristics:
hot-first recursion, bounded offset candidate search over recent 256-unit blocks,
and cache-line-aware scoring of valid offsets. It is opt-in until benchmarks on
full Sudachi dictionaries show a stable lookup win without unacceptable trie size
or build-time regressions.

Run the local harness with:

```bash
cargo bench -p sudachi --bench trie_layout
```

The benchmark compares these variants:

```text
classic_yada
cache_aware_uniform
cache_aware_prefix
cache_aware_external_profile
```

These are the #117 production-compatible variants: every one emits the current
Yada/Darts `u32` unit layout and is readable by the existing runtime trie.

It validates common-prefix output against `classic_yada` before timing lookup,
prints dictionary/trie byte sizes, and measures:

```text
trie_layout/build
trie_layout/common_prefix_hit
trie_layout/common_prefix_miss
trie_layout/common_prefix_mixed
```

Use real dictionary sources by passing environment variables:

```bash
SUDACHI_TRIE_BENCH_MATRIX=path/to/matrix.def \
SUDACHI_TRIE_BENCH_LEXICONS="path/to/lex.csv:path/to/lex2.csv" \
SUDACHI_TRIE_BENCH_INPUTS=path/to/tokenization-inputs.txt \
SUDACHI_TRIE_BENCH_PROFILE=path/to/profile.tsv \
cargo bench -p sudachi --bench trie_layout
```

`SUDACHI_TRIE_BENCH_LEXICON` can be used instead of
`SUDACHI_TRIE_BENCH_LEXICONS` for a single CSV file. If no profile is supplied,
the benchmark generates a deterministic synthetic profile from indexed forms so
the external-profile code path is still covered.

For a maintainer-facing report, include:

```text
variant, dictionary size, trie size,
build time,
common-prefix hit/miss/mixed time,
benchmark dictionary source,
benchmark input corpus source
```

The proposed merge gate for making cache-aware the default is:

```text
lookup/common-prefix speedup >= 5-10% on realistic core/full workloads
no >2% regression on miss/random workloads
trie size growth <= 3-5%
core dictionary build-time growth <= 10-20%
classic_yada remains available as fallback
```

Suggested PR summary:

```text
This PR adds an opt-in cache-aware, Yada/Darts-compatible trie builder for
dictionary compilation. It preserves the serialized trie format and runtime
lookup semantics, while allowing deterministic profile-aware node placement.
The default remains the existing Yada builder until the benchmark matrix shows
a stable win on real Sudachi dictionaries.
```

For large dictionaries, Criterion's repeated build benchmark can be too
expensive. Use the one-shot report example to collect maintainer-facing numbers:

```bash
SUDACHI_TRIE_BENCH_MATRIX=path/to/matrix.def \
SUDACHI_TRIE_BENCH_LEXICONS="path/to/small_lex.csv:path/to/core_lex.csv" \
cargo run -p sudachi --release --example trie_layout_report
```

It prints a tab-separated table with build time, dictionary size, trie size, and
hit/miss/mixed common-prefix lookup timing for every variant.

For the cross-method maintainer table, use:

```bash
cargo run -p sudachi --release --example dictionary_matcher_report
```

That report compares current Yada, cache-aware Yada variants, Daachorse,
Crawdad, FST, and MARISA-style methods against current Yada for matcher pipeline
speed, memory, serialized size, and result equality. Unlike the pure trie layout
benchmarks, it runs over every UTF-8 character start in each input line and
validates `(start, end, value)` matches.

For the maintainer table, use real tier lexicons with `SUDACHI_TRIE_BENCH_MATRIX`
and `SUDACHI_TRIE_BENCH_LEXICONS`. The current small/core/full results and exact
commands are in `docs/issue-117-performance-report.md`.

By default, the cross-method report prints one row per matcher using the same
`vec_buckets_clear_all` storage. Set `SUDACHI_COMPARE_STORAGE_VARIANTS=1` only
when you explicitly want to study storage strategies as a separate dimension.

Use `SUDACHI_TRIE_BENCH_SURFACE_LEXICONS` only for a faster exploratory run when
you want to isolate index-form trie/matcher cost from full connection-matrix
parsing:

```bash
SUDACHI_TRIE_BENCH_SURFACE_LEXICONS="path/to/small_lex.csv:path/to/core_lex.csv:path/to/notcore_lex.csv" \
SUDACHI_TRIE_BENCH_INPUTS=path/to/inputs.txt \
SUDACHI_COMPARE_INPUT_LIMIT=2048 \
SUDACHI_COMPARE_ROUNDS=100 \
cargo run -p sudachi --release --example dictionary_matcher_report
```

## Current Results

The maintainer-facing conclusion and report fields are kept in
`docs/issue-117-performance-report.md`. This document only describes the
cache-aware trie-layout controls and how to run them.
