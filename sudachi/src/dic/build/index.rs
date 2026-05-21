/*
 *  Copyright (c) 2021-2026 Works Applications Co., Ltd.
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

use crate::dic::build::error::{BuildFailure, DicBuildError};
use crate::dic::word_id::WordId;
use crate::error::{SudachiError, SudachiResult};
use crate::util::fxhash::FxBuildHasher;
use indexmap::map::IndexMap;
use std::path::PathBuf;

mod cache_aware;

const MAX_TRIE_VALUE: u32 = (1 << 31) - 1;

#[derive(Clone, Debug, PartialEq)]
pub enum TrieBuildStrategy {
    ClassicYada,
    CacheAware(CacheAwareOptions),
}

impl Default for TrieBuildStrategy {
    fn default() -> Self {
        Self::ClassicYada
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CacheAwareOptions {
    pub cache_line_bytes: usize,
    pub candidate_window_blocks: usize,
    pub profile_mode: TrieProfileMode,
    pub scoring: LayoutScoring,
}

impl Default for CacheAwareOptions {
    fn default() -> Self {
        Self {
            cache_line_bytes: 64,
            candidate_window_blocks: 16,
            profile_mode: TrieProfileMode::DictionaryPrefixCount,
            scoring: LayoutScoring::default(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TrieProfileMode {
    Uniform,
    DictionaryPrefixCount,
    ExternalKeyProfile(PathBuf),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LayoutScoring {
    pub cache_line_weight: u32,
    pub distance_weight: u32,
    pub spread_weight: u32,
    pub density_weight: u32,
}

impl Default for LayoutScoring {
    fn default() -> Self {
        Self {
            cache_line_weight: 10_000,
            distance_weight: 32,
            spread_weight: 8,
            density_weight: 1,
        }
    }
}

pub struct IndexEntry {
    ids: Vec<WordId>,
    offset: usize,
}

impl Default for IndexEntry {
    fn default() -> Self {
        Self {
            ids: Vec::new(),
            offset: usize::MAX,
        }
    }
}

pub struct IndexBuilder<'a> {
    // Insertion order matters for the built dictionary,
    // so using IndexMap here instead of a simple HashMap
    data: IndexMap<&'a str, IndexEntry, FxBuildHasher>,
    trie_build_strategy: TrieBuildStrategy,
}

impl<'a> IndexBuilder<'a> {
    #[cfg(test)]
    pub fn new() -> Self {
        Self::with_trie_build_strategy(TrieBuildStrategy::default())
    }

    pub fn with_trie_build_strategy(trie_build_strategy: TrieBuildStrategy) -> Self {
        Self {
            data: IndexMap::default(),
            trie_build_strategy,
        }
    }

    pub fn add(&mut self, key: &'a str, id: WordId) {
        self.data.entry(key).or_default().ids.push(id)
    }

    pub fn build_word_id_table(&mut self, non_indexed: &[WordId]) -> SudachiResult<Vec<u8>> {
        // by default assume that there will be 3 entries on average
        let mut result =
            Vec::with_capacity((self.data.len() + usize::from(!non_indexed.is_empty())) * 13);
        for (k, entry) in self.data.iter_mut() {
            entry.offset = result.len();
            // clear stored ids memory after use
            let ids = std::mem::take(&mut entry.ids);
            write_delta_varint_word_ids(&mut result, &ids).map_err(|e| {
                SudachiError::DictionaryCompilationError(DicBuildError {
                    cause: e,
                    line: 0,
                    file: format!("<word id table for `{}` has too much entries>", k),
                })
            })?;
        }
        if !non_indexed.is_empty() {
            write_delta_varint_word_ids(&mut result, non_indexed).map_err(|e| {
                SudachiError::DictionaryCompilationError(DicBuildError {
                    cause: e,
                    line: 0,
                    file: "<word id table for non-indexed entries has too much entries>".to_owned(),
                })
            })?;
        }
        Ok(result)
    }

    pub fn build_trie(&mut self) -> SudachiResult<Vec<u8>> {
        let trie_entries = self.trie_entries()?;
        self.data.clear();
        self.data.shrink_to_fit();

        match &self.trie_build_strategy {
            TrieBuildStrategy::ClassicYada => {
                let trie_entries: Vec<_> = trie_entries
                    .iter()
                    .map(|(key, value)| (key.as_str(), *value))
                    .collect();
                let trie = yada::builder::DoubleArrayBuilder::build(&trie_entries);
                match trie {
                    Some(t) => Ok(t),
                    None => Err(DicBuildError {
                        file: "<trie>".to_owned(),
                        line: 0,
                        cause: BuildFailure::TrieBuildFailure,
                    }
                    .into()),
                }
            }
            TrieBuildStrategy::CacheAware(options) => {
                let trie_entries: Vec<_> = trie_entries
                    .iter()
                    .map(|(key, value)| (key.as_str(), *value))
                    .collect();
                cache_aware::CacheAwareDartsBuilder::build(&trie_entries, options.clone()).map_err(
                    |cause| {
                        DicBuildError {
                            file: "<trie>".to_owned(),
                            line: 0,
                            cause,
                        }
                        .into()
                    },
                )
            }
        }
    }

    fn trie_entries(&self) -> SudachiResult<Vec<(String, u32)>> {
        let mut trie_entries = Vec::new();
        for (k, v) in self.data.iter() {
            if v.offset > u32::MAX as _ {
                return Err(DicBuildError {
                    file: format!("entry {k}"),
                    line: 0,
                    cause: BuildFailure::WordIdTableNotBuilt,
                }
                .into());
            }
            if v.offset as u32 > MAX_TRIE_VALUE {
                return Err(DicBuildError {
                    file: format!("entry {k}"),
                    line: 0,
                    cause: BuildFailure::TrieValueLimitExceeded {
                        entry: (*k).to_owned(),
                        value: v.offset as u32,
                    },
                }
                .into());
            }
            trie_entries.push(((*k).to_owned(), v.offset as u32));
        }
        trie_entries.sort_by(|(a, _), (b, _)| a.cmp(b));
        Ok(trie_entries)
    }
}

fn write_delta_varint_word_ids(dst: &mut Vec<u8>, ids: &[WordId]) -> Result<(), BuildFailure> {
    write_varint32(dst, ids.len() as u32);
    let mut prev = 0u32;
    for wid in ids {
        let current = wid.entry().as_raw();
        let delta = current.saturating_sub(prev);
        write_varint32(dst, delta);
        prev = current;
    }
    Ok(())
}

fn write_varint32(dst: &mut Vec<u8>, mut value: u32) {
    loop {
        if (value & !0x7f) == 0 {
            dst.push(value as u8);
            return;
        }
        dst.push((value as u8 & 0x7f) | 0x80);
        value >>= 7;
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::dic::lexicon::trie::{Trie, TrieEntry};
    use crate::dic::lexicon::word_id_table::WordIdTable;
    use crate::dic::word_id::EntryId;
    use std::convert::TryInto;

    fn make_trie(data: Vec<u8>) -> Trie<'static> {
        let mut elems: Vec<u32> = Vec::with_capacity(data.len() / 4);
        for i in (0..data.len()).step_by(4) {
            let arr: [u8; 4] = data[i..i + 4].try_into().unwrap();
            elems.push(u32::from_le_bytes(arr))
        }
        Trie::new_owned(elems)
    }

    #[test]
    fn build_index_1() {
        let mut bldr = IndexBuilder::new();
        bldr.add("test", WordId::new(0, 0));
        let _ = bldr.build_word_id_table(&[]).unwrap();

        let trie = make_trie(bldr.build_trie().unwrap());
        let mut iter = trie.common_prefix_iterator(b"test", 0);
        assert_eq!(iter.next(), Some(TrieEntry { value: 0, end: 4 }));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn build_index_2() {
        let mut bldr = IndexBuilder::new();
        bldr.add("test", WordId::new(0, 0));
        bldr.add("tes", WordId::new(0, 1));
        let _ = bldr.build_word_id_table(&[]).unwrap();

        let trie = make_trie(bldr.build_trie().unwrap());
        let mut iter = trie.common_prefix_iterator(b"test", 0);
        assert_eq!(iter.next(), Some(TrieEntry { value: 2, end: 3 }));
        assert_eq!(iter.next(), Some(TrieEntry { value: 0, end: 4 }));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn word_id_table_includes_non_indexed_entries() {
        let mut bldr = IndexBuilder::new();
        bldr.add("a", WordId::new(0, 4));
        bldr.add("b", WordId::new(0, 8));

        let table = bldr
            .build_word_id_table(&[WordId::new(0, 12), WordId::new(0, 16)])
            .unwrap();

        let all: Vec<_> = WordIdTable::from_bytes(&table).all_entries().collect();
        assert_eq!(
            all,
            vec![
                EntryId::new(4),
                EntryId::new(8),
                EntryId::new(12),
                EntryId::new(16),
            ]
        );
    }

    fn trie_for_strategy(strategy: TrieBuildStrategy) -> Trie<'static> {
        let entries = [
            ("a", 0),
            ("ab", 1),
            ("aba", 2),
            ("ac", 3),
            ("ba", 4),
            ("京都", 5),
            ("京都府", 6),
            ("東京都", 7),
            ("東京", 8),
        ];
        let mut bldr = IndexBuilder::with_trie_build_strategy(strategy);
        for (key, raw_id) in entries {
            bldr.add(key, WordId::new(0, raw_id));
        }
        let _ = bldr.build_word_id_table(&[]).unwrap();
        make_trie(bldr.build_trie().unwrap())
    }

    #[test]
    fn cache_aware_trie_matches_classic_lookup_semantics() {
        let classic = trie_for_strategy(TrieBuildStrategy::ClassicYada);
        let cache_aware =
            trie_for_strategy(TrieBuildStrategy::CacheAware(CacheAwareOptions::default()));

        for input in [
            b"abacus".as_slice(),
            b"ac".as_slice(),
            b"bad".as_slice(),
            "京都府庁".as_bytes(),
            "東京都".as_bytes(),
            "東京湾".as_bytes(),
        ] {
            let expected: Vec<_> = classic.common_prefix_iterator(input, 0).collect();
            let actual: Vec<_> = cache_aware.common_prefix_iterator(input, 0).collect();
            assert_eq!(actual, expected, "input={:?}", input);
        }
    }

    #[test]
    fn cache_aware_trie_build_is_deterministic() {
        let mut first = IndexBuilder::with_trie_build_strategy(TrieBuildStrategy::CacheAware(
            CacheAwareOptions::default(),
        ));
        let mut second = IndexBuilder::with_trie_build_strategy(TrieBuildStrategy::CacheAware(
            CacheAwareOptions::default(),
        ));
        for (key, raw_id) in [
            ("alpha", 0),
            ("alphabet", 1),
            ("alpine", 2),
            ("beta", 3),
            ("betamax", 4),
            ("京都", 5),
        ] {
            first.add(key, WordId::new(0, raw_id));
            second.add(key, WordId::new(0, raw_id));
        }
        let _ = first.build_word_id_table(&[]).unwrap();
        let _ = second.build_word_id_table(&[]).unwrap();

        assert_eq!(first.build_trie().unwrap(), second.build_trie().unwrap());
    }

    #[test]
    fn cache_aware_trie_accepts_external_profile() {
        use std::io::Write;

        let mut profile = tempfile::NamedTempFile::new().unwrap();
        writeln!(profile, "ab\t100").unwrap();
        writeln!(profile, "hex:E4BAACE983BD\t50").unwrap();

        let options = CacheAwareOptions {
            profile_mode: TrieProfileMode::ExternalKeyProfile(profile.path().to_owned()),
            ..CacheAwareOptions::default()
        };
        let trie = trie_for_strategy(TrieBuildStrategy::CacheAware(options));
        let actual: Vec<_> = trie
            .common_prefix_iterator("京都府".as_bytes(), 0)
            .collect();

        assert_eq!(actual.len(), 2);
    }
}
