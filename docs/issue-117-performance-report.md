# Issue #117 Trie Layout And Matcher Comparison

Issue #117 asks whether Sudachi can build a `darts-clone`/Yada-compatible trie
whose node placement is friendlier to speculative cache prefetch. This report
keeps that scope: trie layout and dictionary matcher candidates only. It does
not cover tokenizer rewrites, sentence splitting, connection-matrix locality, or
dictionary format changes.

The production default remains `classic_yada`. Cache-aware Yada is selected only
with `--trie-layout cache-aware`.

## Benchmark Shape

The maintainer-facing comparison is produced by
`sudachi/examples/dictionary_matcher_report.rs`.

The measured workload matches Sudachi's dictionary matcher shape:

- `classic_yada` and cache-aware Yada run common-prefix lookup at every UTF-8
  character start.
- Daachorse runs one-pass overlapping search over the whole input line and maps
  matches back to the same per-start buckets.
- Crawdad, FST, and MARISA-style methods run prefix lookup at every character
  start through adapters.
- Every candidate is validated against `classic_yada` as `(start, end, value)`.

The table below uses the same storage strategy for every row:
`vec_buckets_clear_all`. That keeps the main table focused on matcher/trie
methods instead of mixing storage experiments into the primary comparison.
Storage variants can still be enabled with `SUDACHI_COMPARE_STORAGE_VARIANTS=1`.

`serialized_bytes` is the serialized matcher/trie size. For Yada this is the
serialized trie bytes, not the whole compiled dictionary.

## Reproduce

All numbers below were generated on 2026-05-21 with real SudachiDict tier files
and the full `matrix.def`:

```bash
export SUDACHI_TRIE_BENCH_MATRIX=target/bench-lookup/raw/unzipped/matrix/matrix.def
export SUDACHI_TRIE_BENCH_INPUTS=target/issue-117-corpora/kyoto-leads.txt
export SUDACHI_COMPARE_INPUT_LIMIT=2048
export SUDACHI_COMPARE_ROUNDS=20

SUDACHI_TRIE_BENCH_DATA_SET_NAME=small \
SUDACHI_TRIE_BENCH_LEXICONS=target/bench-lookup/raw/unzipped/small/small_lex.csv \
cargo run -p sudachi --release --example dictionary_matcher_report --quiet

SUDACHI_TRIE_BENCH_DATA_SET_NAME=core \
SUDACHI_TRIE_BENCH_LEXICONS=target/bench-lookup/raw/unzipped/small/small_lex.csv:target/bench-lookup/raw/unzipped/core/core_lex.csv \
cargo run -p sudachi --release --example dictionary_matcher_report --quiet

SUDACHI_TRIE_BENCH_DATA_SET_NAME=full \
SUDACHI_TRIE_BENCH_LEXICONS=target/bench-lookup/raw/unzipped/small/small_lex.csv:target/bench-lookup/raw/unzipped/core/core_lex.csv:target/bench-lookup/raw/unzipped/notcore/notcore_lex.csv \
cargo run -p sudachi --release --example dictionary_matcher_report --quiet
```

Input corpus for each run: 2,048 Kyoto lead lines, 60,840 UTF-8 character starts
per batch, 20 rounds. All successful rows had `same_order=yes`,
`same_results=yes`, and `mismatches=0`.

## Results

| Dictionary | Variant | Family | Build ms | Match ns/batch | ns/start | Speed vs current | Total heap | Heap vs current | Serialized bytes | Same results | Mismatches |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---|---:|
| small | `classic_yada` | yada | 5886.369 | 1575754 | 25.90 | 1.00x | 8377792 | 1.00x | 8368128 | yes | 0 |
| small | `cache_aware_uniform` | yada | 64045.791 | 1496145 | 24.59 | 1.05x | 8744384 | 1.04x | 8734720 | yes | 0 |
| small | `cache_aware_prefix` | yada | 58393.045 | 1510870 | 24.83 | 1.04x | 8553920 | 1.02x | 8544256 | yes | 0 |
| small | `cache_aware_external_profile` | yada | 58696.148 | 1574114 | 25.87 | 1.00x | 8556992 | 1.02x | 8547328 | yes | 0 |
| small | `daachorse_bytewise` | aho-corasick | 227.398 | 1937047 | 31.84 | 0.81x | 25073404 | 2.99x | 25063753 | yes | 0 |
| small | `daachorse_charwise` | aho-corasick | 1170.017 | 1268062 | 20.84 | 1.24x | 22534340 | 2.69x | 22524697 | yes | 0 |
| small | `crawdad_trie` | charwise-dat | 1021.840 | 1367814 | 22.48 | 1.15x | 8553352 | 1.02x | 8543700 | yes | 0 |
| small | `crawdad_mptrie` | charwise-dat | 1531.722 | 1458187 | 23.97 | 1.08x | 9881164 | 1.18x | 9871518 | yes | 0 |
| small | `fst_map_prefix_scan` | fst | 67.716 | 64928737 | 1067.20 | 0.02x | 2625928 | 0.31x | 2616264 | yes | 0 |
| small | `rsmarisa_trie` | louds | 104.479 | 14944504 | 245.64 | 0.11x | 1720974 | 0.21x | 1709536 | yes | 0 |
| core | `classic_yada` | yada | 13707.006 | 1943222 | 31.94 | 1.00x | 27558592 | 1.00x | 27548672 | yes | 0 |
| core | `cache_aware_uniform` | yada | 206526.321 | 1866585 | 30.68 | 1.04x | 28161728 | 1.02x | 28151808 | yes | 0 |
| core | `cache_aware_prefix` | yada | 178374.356 | 1929785 | 31.72 | 1.01x | 27852480 | 1.01x | 27842560 | yes | 0 |
| core | `cache_aware_external_profile` | yada | 177870.293 | 1847137 | 30.36 | 1.05x | 27851456 | 1.01x | 27841536 | yes | 0 |
| core | `daachorse_bytewise` | aho-corasick | 895.449 | 2954200 | 48.56 | 0.66x | 82576784 | 3.00x | 82566877 | yes | 0 |
| core | `daachorse_charwise` | aho-corasick | 2365.312 | 1641943 | 26.99 | 1.18x | 58892120 | 2.14x | 58882221 | yes | 0 |
| core | `crawdad_trie` | charwise-dat | 1074.908 | 1544922 | 25.39 | 1.26x | 21660808 | 0.79x | 21650900 | yes | 0 |
| core | `crawdad_mptrie` | charwise-dat | 3003.973 | 1619154 | 26.61 | 1.20x | 23907456 | 0.87x | 23897554 | yes | 0 |
| core | `fst_map_prefix_scan` | fst | 261.432 | 70462727 | 1158.16 | 0.03x | 10728629 | 0.39x | 10718709 | yes | 0 |
| core | `rsmarisa_trie` | louds | 443.588 | 15884666 | 261.09 | 0.12x | 4987247 | 0.18x | 4975536 | yes | 0 |
| full | `classic_yada` | yada | 41394.199 | 2250658 | 36.99 | 1.00x | 72173696 | 1.00x | 72163328 | yes | 0 |
| full | `cache_aware_uniform` | yada | 613334.868 | 2112645 | 34.72 | 1.07x | 73933952 | 1.02x | 73923584 | yes | 0 |
| full | `cache_aware_prefix` | yada | 561598.820 | 2063293 | 33.91 | 1.09x | 73402496 | 1.02x | 73392128 | yes | 0 |
| full | `cache_aware_external_profile` | yada | 560395.539 | 2050664 | 33.71 | 1.10x | 73401472 | 1.02x | 73391104 | yes | 0 |
| full | `daachorse_bytewise` | aho-corasick | 3091.131 | 2849387 | 46.83 | 0.79x | 215297532 | 2.98x | 215287177 | yes | 0 |
| full | `daachorse_charwise` | aho-corasick | 4949.957 | 1730627 | 28.45 | 1.30x | 134182852 | 1.86x | 134172505 | yes | 0 |
| full | `crawdad_trie` | charwise-dat | 1296.260 | 1723370 | 28.33 | 1.31x | 54756936 | 0.76x | 54746580 | yes | 0 |
| full | `crawdad_mptrie` | charwise-dat | 7519.756 | 1773652 | 29.15 | 1.27x | 48338934 | 0.67x | 48328584 | yes | 0 |
| full | `fst_map_prefix_scan` | fst | 736.915 | 76801166 | 1262.35 | 0.03x | 29389330 | 0.41x | 29378962 | yes | 0 |
| full | `rsmarisa_trie` | louds | 1068.786 | 18183977 | 298.88 | 0.12x | 11188924 | 0.16x | 11176784 | yes | 0 |

## Interpretation

Cache-aware Yada is the only option here that preserves the current Yada/Darts
serialized trie format. It reaches the 5-10% lookup-speed target only on the
full tier (`1.09x` to `1.10x`), but build time is far outside the acceptable
range: `41.394 s` for classic full vs about `560-613 s` for cache-aware full.
Trie size grows by about 1-2%. That is not a default-worthy tradeoff yet.

Daachorse charwise one-pass is consistently faster than current Yada and keeps
exact result parity, but it is not Yada-compatible and uses materially more
memory: `1.86x` total heap on full.

Crawdad is the strongest non-Yada comparison in this run: it is faster than
current Yada, exact-result compatible through the adapter, and more compact on
core/full. It is still a runtime/backend replacement decision, not an issue #117
layout-only change.

FST and MARISA are compact, but the prefix-scan adapters are much slower than
current Yada on this workload.

## Decision Gate

Keep `classic_yada` as the default for issue #117 unless a real small/core/full
workload shows:

- Yada-compatible lookup speedup of at least 5-10%;
- no more than 2% regression on miss-heavy workloads;
- trie size growth below 3-5%;
- core/full dictionary build-time growth below 10-20%;
- `same_results=yes` for all benchmarked inputs.

Based on the full small/core/full runs above, cache-aware Yada should remain
opt-in. The bigger speed opportunities are separate matcher/backend decisions,
not just a prefetch-compatible Yada layout change.
