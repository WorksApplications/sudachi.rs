# Sudachi.rs tokenization performance investigation

A measured study of where tokenization time goes and which optimizations pay
off. The headline result of issue #117 (runtime prefetch trie) lives in
`docs/trie-prefetch.md`; this document is the broader investigation that frames
it, including the negative results, so the trade-offs are explicit.

## Methodology

- **Hardware:** Apple M4 Max (12 performance + 4 efficiency cores), rustc 1.95.0,
  `--release`.
- **Dictionaries:** SudachiDict tiers built with the current `sudachi build`
  from the real `matrix.def` (5981×5981, 71 MB connection matrix, shared by all
  tiers): small (8.4 MB trie), core (27.5 MB), full (72 MB).
- **Corpus:** 16,051 Kyoto lead sentences (461,815 chars), Mode C.
- **Protocol:** end-to-end = reset → `do_tokenize` → `collect_results`
  (`InfoSubset::all`). 31 trials, scalar and pipelined paths interleaved per
  trial to avoid cache-warming bias. We report **median** (robust) and the
  **coefficient of variation (CV)**; a win is credible only when it clears the
  CV noise floor (~1.7% here). Best-of-N is reported too but is noisier.
- **Reproduce:** `examples/tokenize_pipeline_bench` (single-thread A/B),
  `examples/tokenize_parallel_bench` (scaling), `examples/dictionary_matcher_report`
  (isolated lookup, `--features matcher-comparison`).

## Where the time goes (production, pipelined path)

`sample`-based profile of the pipelined tokenizer, full dict, self-time share of
10,871 samples:

| Subsystem | Share | Note |
|---|---:|---|
| Connection matrix (`Lattice::insert` / Viterbi) | **22.8%** | scattered reads into the 71 MB matrix, one per node pair |
| Trie walk (`Trie::batch_impl`) | **16.9%** | the common-prefix search (already pipelined, #117) |
| Allocations (malloc/free/realloc/memmove) | 9.6% | nodes, result strings, WordInfo |
| WordInfo decode + UTF-16→UTF-8 | 9.6% | only paid when results are materialized |
| word-id table varint expansion | 9.5% | delta-decompression of trie leaves → word ids |
| OOV providers (MeCab + aho-corasick) | 7.5% | per boundary |
| Input build / normalization | 4.3% | |
| node build, path rewrite, backtrace, tail | ~20% | |

Two facts drive everything below: the **matrix dominates** and the **trie walk
is second** but already optimized.

## Results per optimization

### 1. Runtime prefetch trie lookup (issue #117) — SHIPPED, +~9%

Software-pipelined + prefetched common-prefix search (see `docs/trie-prefetch.md`).
End-to-end, median of 31 trials, CV ≤ 1.9%:

| Tier | scalar (median) | pipelined (median) | speedup |
|---|---:|---:|---:|
| small | 182.83 ms | 168.30 ms | **1.086x** |
| core | 189.63 ms | 174.77 ms | **1.085x** |
| full | 197.52 ms | 180.46 ms | **1.095x** |

A stable **+8.5–9.5%** across tiers, well above the ~1.7% noise floor, with
byte-identical output, zero build/size/memory cost, and no format change.

Isolated lookup (warm-cache micro-benchmark, K=4) is more modest — small 0.95x,
core 1.07x, full 1.08x — because repeating lookups over the same inputs keeps the
trie hot, so there is little latency to hide. End-to-end is larger because the
trie competes with the 71 MB matrix for cache, exposing the misses the prefetch
covers. The no-prefetch "pipelining only" variant is slower than scalar on every
tier, so the explicit prefetch is doing the work.

### 2. Multithreaded batch tokenization — the big lever, ~9× / ~10×

Sentences are independent and the dictionary is `Send + Sync` (shared via `Arc`);
each worker owns a `StatefulTokenizer`. Full dict, best-of-11:

| Threads | sent/s | speedup | efficiency |
|---:|---:|---:|---:|
| 1 | 90,418 | 1.00x | 100% |
| 2 | 176,706 | 1.95x | 98% |
| 4 | 326,783 | 3.61x | 90% |
| 8 | 630,763 | 6.98x | 87% |
| 16 | 802,669 | **8.88x** | 56% |

Near-linear through the 12 performance cores; the tail is the 4 efficiency cores.
Combined with the pipeline, **~9.9× vs the original single-thread scalar**
(802k vs ~81k sent/s). Morpheme counts are identical at every thread count.
This is the dominant win for batch workloads and stacks multiplicatively with
every per-pipeline optimization.

### 3. Connection-matrix access — NEGATIVE result, do not prefetch

The matrix is the #1 hotspot (22.8%), so it was the obvious target. Two attempts,
both measured:

- **Software prefetch of upcoming matrix entries in `connect_node`** — *hurt*:
  pipelined full regressed from ~181 ms to ~202 ms. All predecessors of a node
  read the *same* 12 KB matrix row (fixed `left_id`), which is largely
  L2-resident, so there is little miss latency to hide; the per-predecessor
  prefetch instructions and bookkeeping are pure overhead in this very hot loop.
- **Hoisting the row base out of the loop** — *neutral*: LLVM already hoists the
  loop-invariant `right * num_left`, so there was nothing to gain.

The matrix cost is **volume-bound, not miss-bound**. Reducing it requires fewer
lookups (beam/pruning), which is an approximation and changes results — out of
scope for exact parity. **Recommendation: leave `connect_node` as-is.**

### 4. Alternative trie backend (Crawdad / Daachorse) — low end-to-end ROI

The cross-method report shows charwise Daachorse / Crawdad are 18–31% faster than
Yada *on the isolated lookup*. But the trie walk is only 16.9% of end-to-end
time, so replacing it buys ~**+3–5% end-to-end** — while changing the serialized
dictionary format and breaking compatibility. Not worth it unless a workload is
specifically lookup-dominated.

### 5. Lookup de-iteratorization + faster varint — mostly done / minor

The `flat_map` iterator overhead seen in mixed profiles comes mainly from the
scalar path; the shipped pipelined `lookup_batch` already replaces it with a
tight loop. The remaining lever is the varint expansion (~9.5%); a branchless
LEB128 decoder might add ~+3% end-to-end, format-preserving. Modest.

## Conclusions and recommendations

- **Batch throughput: multithreading is the answer (~9–10×).** The library is
  already thread-safe; the example documents the pattern. This dwarfs every
  per-op optimization and composes with them.
- **Single-thread latency is near its practical limit for *exact*
  tokenization.** The +9% prefetch pipeline (#117) is most of the available
  format-preserving gain. The dominant remaining cost (the Viterbi connection
  matrix, ~23%) is volume-bound and cannot be reduced without approximate search.
- **Negative results to respect:** matrix prefetch hurts; matrix row-hoist is a
  no-op; an alternative trie backend is only ~+4% end-to-end for a format break.
- **Optional small wins (format-preserving):** branchless varint (~+3%),
  allocation reuse (a few %, diffuse), and lazy/subset `WordInfo` for
  surface-only workloads.

The recommended, shippable package: the #117 runtime prefetch pipeline (default
on, +~9%, exact) plus documenting/benchmarking the ~10× batch parallelism.
