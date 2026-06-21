/*
 * Copyright (c) 2021-2026 Works Applications Co., Ltd.
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

use crate::util::cow_array::CowArray;
use crate::util::prefetch::prefetch_l1;
use std::iter::FusedIterator;

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct TrieEntry {
    /// Value of Trie, this is not the pointer to WordId, but the offset in WordId table
    pub value: u32,
    /// Offset of word end
    pub end: usize,
}

impl TrieEntry {
    #[inline]
    pub fn new(value: u32, offset: usize) -> TrieEntry {
        TrieEntry { value, end: offset }
    }
}

pub struct Trie<'a> {
    array: CowArray<'a, u32>,
}

pub struct TrieEntryIter<'a> {
    trie: &'a [u32],
    node_pos: usize,
    data: &'a [u8],
    offset: usize,
}

impl<'a> Iterator for TrieEntryIter<'a> {
    type Item = TrieEntry;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let mut node_pos = self.node_pos;

        for i in self.offset..self.data.len() {
            // Unwrap is safe: access is always in bounds
            // It is optimized away: https://rust.godbolt.org/z/va9K3az4n
            let k = *self.data.get(i).unwrap();
            match step_once(self.trie, k, &mut node_pos) {
                Step::Dead => return None,
                Step::Continue => {}
                Step::Match { value } => {
                    let r = TrieEntry::new(value, i + 1);
                    self.offset = r.end;
                    self.node_pos = node_pos;
                    return Some(r);
                }
            }
        }
        None
    }
}

impl FusedIterator for TrieEntryIter<'_> {}

/// Outcome of consuming one input byte during a double-array trie walk.
enum Step {
    Dead,
    Continue,
    Match { value: u32 },
}

/// Consume one input byte `k` from the node at `node_pos`, advancing it. Shared
/// by [`TrieEntryIter`] and [`Trie::common_prefix_batch`] so the scalar and
/// pipelined walks are identical by construction.
#[inline(always)]
fn step_once(trie: &[u32], k: u8, node_pos: &mut usize) -> Step {
    let k = k as usize;
    *node_pos ^= k;
    // Safe: the trie is built so every reachable index is in bounds.
    let unit = *unsafe { trie.get_unchecked(*node_pos) } as usize;
    if Trie::label(unit) != k {
        return Step::Dead;
    }
    *node_pos ^= Trie::offset(unit);
    if Trie::has_leaf(unit) {
        Step::Match {
            value: Trie::value(*unsafe { trie.get_unchecked(*node_pos) }),
        }
    } else {
        Step::Continue
    }
}

/// Speculatively prefetch the cache line a walk will load next: `node_pos ^
/// input[pos]` is the index `step_once` will dereference on this lane's next
/// visit. The index is clamped; the prefetch is only a hint, so a wrong address
/// is harmless.
#[inline(always)]
fn prefetch_lane(trie: &[u32], input: &[u8], node_pos: usize, pos: usize) {
    if pos < input.len() {
        let k = *unsafe { input.get_unchecked(pos) } as usize;
        let idx = (node_pos ^ k).min(trie.len().saturating_sub(1));
        prefetch_l1(unsafe { trie.as_ptr().add(idx) });
    }
}

impl<'a> Trie<'a> {
    /// Number of independent walks [`Trie::common_prefix_batch`] keeps in flight.
    pub const DEFAULT_PREFETCH_LANES: usize = 4;
    /// Upper bound on lanes so the scheduler can keep lane state on the stack.
    pub const MAX_PREFETCH_LANES: usize = 16;

    pub fn from_bytes(data: &'a [u8]) -> Trie<'a> {
        Trie {
            array: CowArray::from_bytes(data, 0, data.len() / std::mem::size_of::<u32>()),
        }
    }

    pub fn new(data: &'a [u8], size: usize) -> Trie<'a> {
        Trie {
            array: CowArray::from_bytes(data, 0, size),
        }
    }

    pub fn new_owned(data: Vec<u32>) -> Trie<'a> {
        Trie {
            array: CowArray::from_owned(data),
        }
    }

    pub fn total_size(&self) -> usize {
        4 * self.array.len()
    }

    #[inline]
    pub fn common_prefix_iterator<'b>(&'a self, input: &'b [u8], offset: usize) -> TrieEntryIter<'b>
    where
        'a: 'b,
    {
        let unit: usize = self.get(0) as usize;

        TrieEntryIter {
            node_pos: Trie::offset(unit),
            data: input,
            trie: &self.array,
            offset,
        }
    }

    /// Run common-prefix search from many start positions, overlapping their
    /// memory latency.
    ///
    /// `emit(bucket, value, end)` fires once per match; `bucket` indexes
    /// `starts`. Within a bucket, matches keep [`Trie::common_prefix_iterator`]
    /// order, so grouping by `bucket` reproduces the scalar result.
    #[inline]
    pub fn common_prefix_batch<F: FnMut(usize, u32, usize)>(
        &self,
        input: &[u8],
        starts: &[usize],
        emit: F,
    ) {
        self.common_prefix_batch_cfg(input, starts, Self::DEFAULT_PREFETCH_LANES, true, emit)
    }

    /// [`Trie::common_prefix_batch`] with an explicit lane count and prefetch
    /// toggle. `lanes` is rounded to the nearest compile-time-specialized count;
    /// values above [`Trie::MAX_PREFETCH_LANES`] are clamped.
    #[inline]
    pub fn common_prefix_batch_cfg<F: FnMut(usize, u32, usize)>(
        &self,
        input: &[u8],
        starts: &[usize],
        lanes: usize,
        prefetch: bool,
        emit: F,
    ) {
        // Specialize on lanes and prefetch so both are compile-time constants.
        macro_rules! dispatch {
            ($k:literal) => {
                if prefetch {
                    self.batch_impl::<$k, true, F>(input, starts, emit)
                } else {
                    self.batch_impl::<$k, false, F>(input, starts, emit)
                }
            };
        }
        match lanes {
            0..=2 => dispatch!(2),
            3..=4 => dispatch!(4),
            5..=6 => dispatch!(6),
            7..=8 => dispatch!(8),
            9..=12 => dispatch!(12),
            _ => dispatch!(16),
        }
    }

    fn batch_impl<const K: usize, const PF: bool, F: FnMut(usize, u32, usize)>(
        &self,
        input: &[u8],
        starts: &[usize],
        mut emit: F,
    ) {
        if starts.is_empty() {
            return;
        }
        let trie: &[u32] = &self.array;
        let root = Trie::offset(self.get(0) as usize);
        let n_in = input.len();

        // Per-lane state, structure-of-arrays for constant `K`.
        let mut node = [0usize; K];
        let mut pos = [0usize; K];
        let mut bucket = [0usize; K];
        let mut active = [false; K];
        let mut next_start = 0usize;
        let mut live = 0usize;

        for i in 0..K {
            if next_start < starts.len() {
                node[i] = root;
                pos[i] = starts[next_start];
                bucket[i] = next_start;
                active[i] = true;
                next_start += 1;
                live += 1;
                if PF {
                    prefetch_lane(trie, input, root, pos[i]);
                }
            }
        }

        while live > 0 {
            for i in 0..K {
                if !active[i] {
                    continue;
                }
                let advanced = if pos[i] < n_in {
                    let k = *unsafe { input.get_unchecked(pos[i]) };
                    let mut np = node[i];
                    match step_once(trie, k, &mut np) {
                        Step::Dead => false,
                        Step::Continue => {
                            node[i] = np;
                            pos[i] += 1;
                            if PF {
                                prefetch_lane(trie, input, np, pos[i]);
                            }
                            true
                        }
                        Step::Match { value } => {
                            let end = pos[i] + 1;
                            emit(bucket[i], value, end);
                            node[i] = np;
                            pos[i] = end;
                            if PF {
                                prefetch_lane(trie, input, np, end);
                            }
                            true
                        }
                    }
                } else {
                    false
                };

                if !advanced {
                    if next_start < starts.len() {
                        node[i] = root;
                        pos[i] = starts[next_start];
                        bucket[i] = next_start;
                        next_start += 1;
                        if PF {
                            prefetch_lane(trie, input, root, pos[i]);
                        }
                    } else {
                        active[i] = false;
                        live -= 1;
                    }
                }
            }
        }
    }

    #[inline(always)]
    fn get(&self, index: usize) -> u32 {
        debug_assert!(index < self.array.len());
        // UB if out of bounds
        // Should we panic in release builds here instead?
        // Safe version is not optimized away
        *unsafe { self.array.get_unchecked(index) }
    }

    #[inline(always)]
    fn has_leaf(unit: usize) -> bool {
        ((unit >> 8) & 1) == 1
    }

    #[inline(always)]
    fn value(unit: u32) -> u32 {
        unit & ((1 << 31) - 1)
    }

    #[inline(always)]
    fn label(unit: usize) -> usize {
        unit & ((1 << 31) | 0xFF)
    }

    #[inline(always)]
    fn offset(unit: usize) -> usize {
        (unit >> 10) << ((unit & (1 << 9)) >> 6)
    }
}

#[cfg(test)]
mod tests {
    use super::Trie;

    /// Build a double array from `(key, value)` pairs.
    fn build_trie(mut keys: Vec<(&str, u32)>) -> Vec<u8> {
        keys.sort_by(|a, b| a.0.cmp(b.0));
        yada::builder::DoubleArrayBuilder::build(&keys).expect("trie build failed")
    }

    /// Per-start reference output of the scalar iterator.
    fn scalar_reference(trie: &Trie, input: &[u8], starts: &[usize]) -> Vec<Vec<(u32, usize)>> {
        starts
            .iter()
            .map(|&s| {
                trie.common_prefix_iterator(input, s)
                    .map(|e| (e.value, e.end))
                    .collect()
            })
            .collect()
    }

    fn batch_grouped(
        trie: &Trie,
        input: &[u8],
        starts: &[usize],
        lanes: usize,
        prefetch: bool,
    ) -> Vec<Vec<(u32, usize)>> {
        let mut got: Vec<Vec<(u32, usize)>> = vec![Vec::new(); starts.len()];
        trie.common_prefix_batch_cfg(input, starts, lanes, prefetch, |bucket, value, end| {
            got[bucket].push((value, end));
        });
        got
    }

    #[test]
    fn batch_matches_scalar_across_lanes_and_prefetch() {
        // Shared prefixes, a prefix-of-another key, multibyte UTF-8, and misses.
        let bytes = build_trie(vec![
            ("a", 1),
            ("ab", 2),
            ("abc", 3),
            ("abd", 4),
            ("b", 5),
            ("bc", 6),
            ("東", 7),
            ("東京", 8),
            ("東京都", 9),
        ]);
        let trie = Trie::from_bytes(&bytes);

        for text in ["abcabd b 東京都 abZ", "東京", "", "zzz", "ab東京都bca"] {
            let input = text.as_bytes();
            let starts: Vec<usize> = (0..=input.len()).collect();
            let reference = scalar_reference(&trie, input, &starts);
            for &lanes in &[1usize, 2, 3, 8, 16] {
                for &prefetch in &[false, true] {
                    let got = batch_grouped(&trie, input, &starts, lanes, prefetch);
                    assert_eq!(
                        got, reference,
                        "mismatch for text {text:?} lanes={lanes} prefetch={prefetch}"
                    );
                }
            }
        }
    }

    #[test]
    fn batch_handles_sparse_and_unordered_starts() {
        let bytes = build_trie(vec![("ab", 2), ("abc", 3), ("xy", 10)]);
        let trie = Trie::from_bytes(&bytes);
        let input = b"abcxyab";
        // Non-contiguous, repeated, and out-of-order starts.
        let starts = [6usize, 0, 3, 0, 3];
        let reference = scalar_reference(&trie, input, &starts);
        for &lanes in &[1usize, 2, 16] {
            for &prefetch in &[false, true] {
                let got = batch_grouped(&trie, input, &starts, lanes, prefetch);
                assert_eq!(got, reference, "lanes={lanes} prefetch={prefetch}");
            }
        }
    }
}
