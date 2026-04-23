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

use crate::dic::build::error::{BuildFailure, DicWriteResult};
use crate::dic::build::lexicon::{ParsedLexiconEntry, ResolvedLexiconEntry, WordRefResolver};
use crate::dic::subset::InfoSubset;
use crate::dic::word_id::{DictId, WordRef};
use crate::dic::word_info::WordInfo;
use crate::dic::{DictionaryAccess, ReferenceIdAccess};
use crate::error::SudachiResult;
use crate::util::fxhash::FxBuildHasher;
use std::collections::HashMap;

type HeadwordIndex = HashMap<String, Vec<ResolutionCandidate>, FxBuildHasher>;
type ReferenceIdIndex = HashMap<String, ResolutionCandidate, FxBuildHasher>;

#[derive(Clone)]
struct ResolutionCandidate {
    headword: String,
    pos_id: u16,
    reading: Option<String>,
    wref: WordRef,
}

pub(crate) trait ResolverEntryView {
    fn headword(&self) -> &str;
    fn reading(&self) -> &str;
    fn pos_id(&self) -> u16;
    fn reference_id(&self) -> Option<&str>;
}

impl ResolverEntryView for ParsedLexiconEntry {
    fn headword(&self) -> &str {
        self.headword()
    }

    fn reading(&self) -> &str {
        self.reading()
    }

    fn pos_id(&self) -> u16 {
        self.pos
    }

    fn reference_id(&self) -> Option<&str> {
        self.reference_id()
    }
}

impl ResolverEntryView for ResolvedLexiconEntry {
    fn headword(&self) -> &str {
        self.headword()
    }

    fn reading(&self) -> &str {
        self.reading()
    }

    fn pos_id(&self) -> u16 {
        self.pos
    }

    fn reference_id(&self) -> Option<&str> {
        self.reference_id()
    }
}

/// Resolver based on a (system) binary dictionary.
///
/// We can't use trie to resolve splits because it is possible that refs are not in trie.
/// This resolver has to be owning because the dictionary content is lazily loaded and transient.
pub struct BinDictResolver<D> {
    dict: D,
    headword_index: HeadwordIndex,
    reference_id_index: HashMap<String, WordRef, FxBuildHasher>,
    line_to_wref: Vec<WordRef>,
}

impl<D: DictionaryAccess + ReferenceIdAccess> BinDictResolver<D> {
    pub fn new(dict: D) -> SudachiResult<Self> {
        let lex = dict.lexicon();
        let line_to_wid = lex.system_word_ids_in_order();
        let line_to_wref = line_to_wid
            .iter()
            .map(|wid| WordRef::new(true, wid.entry().as_raw()))
            .collect::<Vec<_>>();
        let mut headword_index: HeadwordIndex = HashMap::default();
        for wid in line_to_wid.iter().copied() {
            let winfo: WordInfo = lex.get_word_info_subset(
                wid,
                InfoSubset::HEADWORD | InfoSubset::READING_FORM | InfoSubset::POS_ID,
            )?;
            let headword = winfo.headword(&dict).to_string();
            let reading = normalized_reading(&headword, winfo.reading_form(&dict));
            let wref = WordRef::new(true, wid.entry().as_raw());
            headword_index
                .entry(headword.clone())
                .or_default()
                .push(ResolutionCandidate {
                    headword,
                    pos_id: winfo.pos_id(),
                    reading,
                    wref,
                });
        }

        let mut reference_id_index = HashMap::default();
        for (entry_id, reference_id) in dict.reference_ids() {
            reference_id_index.insert(reference_id, WordRef::new(true, entry_id));
        }

        Ok(Self {
            dict,
            headword_index,
            reference_id_index,
            line_to_wref,
        })
    }
}

impl<D: DictionaryAccess + ReferenceIdAccess> WordRefResolver for BinDictResolver<D> {
    fn resolve_by_line_ref(&self, line_ref: WordRef) -> Option<WordRef> {
        if !line_ref.is_system() {
            return None;
        }
        self.line_to_wref
            .get(line_ref.entry().as_raw() as usize)
            .copied()
    }

    fn resolve_by_headword(&self, headword: &str) -> Option<WordRef> {
        self.headword_index
            .get(headword)
            .and_then(|v| v.first().map(|candidate| candidate.wref))
    }

    fn resolve_entry_key(
        &self,
        headword: &str,
        pos: u16,
        reading: Option<&str>,
        reference_id: Option<&str>,
    ) -> Option<WordRef> {
        match reference_id {
            Some(reference_id) => {
                let wref = *self.reference_id_index.get(reference_id)?;
                let wid = wref.resolve(DictId::SYSTEM);
                let winfo = self
                    .dict
                    .lexicon()
                    .get_word_info_subset(
                        wid,
                        InfoSubset::HEADWORD | InfoSubset::READING_FORM | InfoSubset::POS_ID,
                    )
                    .ok()?;
                let actual_headword = winfo.headword(&self.dict);
                let actual_reading =
                    normalized_reading(actual_headword, winfo.reading_form(&self.dict));
                if actual_headword == headword
                    && winfo.pos_id() == pos
                    && actual_reading.as_deref() == reading
                {
                    Some(wref)
                } else {
                    None
                }
            }
            None => self.headword_index.get(headword).and_then(|v| {
                v.iter()
                    .find(|candidate| {
                        candidate.pos_id == pos && candidate.reading.as_deref() == reading
                    })
                    .map(|candidate| candidate.wref)
            }),
        }
    }
}

/// Resolver based on a lexicon csv.
pub struct RawDictResolver {
    headword_index: HeadwordIndex,
    reference_id_index: ReferenceIdIndex,
    line_to_wref: Vec<WordRef>,
    user: bool,
}

impl RawDictResolver {
    pub(crate) fn new<T: ResolverEntryView>(
        entries: &[T],
        line_to_wref: Vec<WordRef>,
        user: bool,
    ) -> DicWriteResult<Self> {
        let mut headword_index: HeadwordIndex = HashMap::default();
        let mut reference_id_index: ReferenceIdIndex = HashMap::default();

        for (i, e) in entries.iter().enumerate() {
            let headword = e.headword().to_owned();
            let wref = line_to_wref[i];
            let candidate = ResolutionCandidate {
                headword: headword.clone(),
                pos_id: e.pos_id(),
                reading: normalized_reading(e.headword(), e.reading()),
                wref,
            };

            if let Some(reference_id) = e.reference_id() {
                if reference_id_index
                    .insert(reference_id.to_owned(), candidate.clone())
                    .is_some()
                {
                    return Err(BuildFailure::InvalidSplit(format!(
                        "duplicated reference_id: {reference_id}"
                    )));
                }
            }

            headword_index.entry(headword).or_default().push(candidate);
        }

        Ok(Self {
            headword_index,
            reference_id_index,
            line_to_wref,
            user,
        })
    }
}

impl WordRefResolver for RawDictResolver {
    fn resolve_by_line_ref(&self, line_ref: WordRef) -> Option<WordRef> {
        if line_ref.is_system() == self.user {
            return None;
        }
        self.line_to_wref
            .get(line_ref.entry().as_raw() as usize)
            .copied()
    }

    fn resolve_by_headword(&self, headword: &str) -> Option<WordRef> {
        self.headword_index
            .get(headword)
            .and_then(|v| v.first().map(|candidate| candidate.wref))
    }

    fn resolve_entry_key(
        &self,
        headword: &str,
        pos: u16,
        reading: Option<&str>,
        reference_id: Option<&str>,
    ) -> Option<WordRef> {
        match reference_id {
            Some(reference_id) => self
                .reference_id_index
                .get(reference_id)
                .and_then(|candidate| {
                    if candidate.headword == headword
                        && candidate.pos_id == pos
                        && candidate.reading.as_deref() == reading
                    {
                        Some(candidate.wref)
                    } else {
                        None
                    }
                }),
            None => self.headword_index.get(headword).and_then(|data| {
                data.iter()
                    .find(|candidate| {
                        candidate.pos_id == pos && candidate.reading.as_deref() == reading
                    })
                    .map(|candidate| candidate.wref)
            }),
        }
    }
}

pub(crate) struct ChainedResolver<A, B> {
    a: A,
    b: B,
}

impl<A: WordRefResolver, B: WordRefResolver> ChainedResolver<A, B> {
    pub(crate) fn new(a: A, b: B) -> Self {
        Self { a, b }
    }
}

impl<A: WordRefResolver, B: WordRefResolver> WordRefResolver for ChainedResolver<A, B> {
    fn resolve_by_line_ref(&self, line_ref: WordRef) -> Option<WordRef> {
        self.a
            .resolve_by_line_ref(line_ref)
            .or_else(|| self.b.resolve_by_line_ref(line_ref))
    }

    fn resolve_by_headword(&self, headword: &str) -> Option<WordRef> {
        self.a
            .resolve_by_headword(headword)
            .or_else(|| self.b.resolve_by_headword(headword))
    }

    fn resolve_entry_key(
        &self,
        headword: &str,
        pos: u16,
        reading: Option<&str>,
        reference_id: Option<&str>,
    ) -> Option<WordRef> {
        self.a
            .resolve_entry_key(headword, pos, reading, reference_id)
            .or_else(|| {
                self.b
                    .resolve_entry_key(headword, pos, reading, reference_id)
            })
    }
}

fn normalized_reading(headword: &str, reading: &str) -> Option<String> {
    if reading.is_empty() || headword == reading {
        None
    } else {
        Some(reading.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dic::build::lexicon::WordRef as BuildWordRef;
    use crate::dic::word_id::WordRef as DicWordRef;

    struct TestEntry {
        headword: &'static str,
        reading: &'static str,
        pos_id: u16,
        reference_id: Option<&'static str>,
    }

    impl ResolverEntryView for TestEntry {
        fn headword(&self) -> &str {
            self.headword
        }

        fn reading(&self) -> &str {
            self.reading
        }

        fn pos_id(&self) -> u16 {
            self.pos_id
        }

        fn reference_id(&self) -> Option<&str> {
            self.reference_id
        }
    }

    struct StubResolver {
        by_line_ref: Option<DicWordRef>,
        by_headword: Option<DicWordRef>,
        by_entry_key: Option<DicWordRef>,
    }

    impl WordRefResolver for StubResolver {
        fn resolve_by_line_ref(&self, _line_ref: DicWordRef) -> Option<DicWordRef> {
            self.by_line_ref
        }

        fn resolve_by_headword(&self, _headword: &str) -> Option<DicWordRef> {
            self.by_headword
        }

        fn resolve_entry_key(
            &self,
            _headword: &str,
            _pos: u16,
            _reading: Option<&str>,
            _reference_id: Option<&str>,
        ) -> Option<DicWordRef> {
            self.by_entry_key
        }
    }

    #[test]
    fn chained_resolver_prioritizes_first_resolver() {
        let first = StubResolver {
            by_line_ref: Some(DicWordRef::new(true, 3)),
            by_headword: Some(DicWordRef::new(true, 1)),
            by_entry_key: Some(DicWordRef::new(true, 2)),
        };
        let second = StubResolver {
            by_line_ref: Some(DicWordRef::new(false, 3)),
            by_headword: Some(DicWordRef::new(false, 1)),
            by_entry_key: Some(DicWordRef::new(false, 2)),
        };
        let chained = ChainedResolver::new(first, second);

        assert_eq!(
            chained.resolve(&BuildWordRef::LineRef(DicWordRef::new(true, 0))),
            Some(DicWordRef::new(true, 3))
        );
        assert_eq!(
            chained.resolve(&BuildWordRef::Headword("京都".to_string())),
            Some(DicWordRef::new(true, 1))
        );
        assert_eq!(
            chained.resolve(&BuildWordRef::EntryKey {
                headword: "京都".to_string(),
                pos: 0,
                reading: Some("キョウト".to_string()),
                reference_id: Some("kyoto".to_string()),
            }),
            Some(DicWordRef::new(true, 2))
        );
    }

    #[test]
    fn raw_resolver_resolves_entry_key_to_first_duplicate_in_csv_order() {
        let entries = vec![
            TestEntry {
                headword: "京都",
                reading: "キョウト",
                pos_id: 0,
                reference_id: None,
            },
            TestEntry {
                headword: "京都",
                reading: "キョウト",
                pos_id: 0,
                reference_id: None,
            },
            TestEntry {
                headword: "京都",
                reading: "ミヤコ",
                pos_id: 1,
                reference_id: None,
            },
        ];
        let line_to_wref = vec![
            DicWordRef::new(true, 11),
            DicWordRef::new(true, 27),
            DicWordRef::new(true, 42),
        ];
        let resolver = RawDictResolver::new(&entries, line_to_wref.clone(), false).unwrap();

        assert_eq!(
            resolver.resolve_entry_key("京都", 0, Some("キョウト"), None),
            Some(line_to_wref[0])
        );
    }

    #[test]
    fn raw_resolver_resolves_headword_to_first_duplicate_in_csv_order() {
        let entries = vec![
            TestEntry {
                headword: "京都",
                reading: "キョウト",
                pos_id: 0,
                reference_id: None,
            },
            TestEntry {
                headword: "京都",
                reading: "ミヤコ",
                pos_id: 1,
                reference_id: None,
            },
            TestEntry {
                headword: "京都",
                reading: "キョウト",
                pos_id: 2,
                reference_id: None,
            },
        ];
        let line_to_wref = vec![
            DicWordRef::new(true, 11),
            DicWordRef::new(true, 27),
            DicWordRef::new(true, 42),
        ];
        let resolver = RawDictResolver::new(&entries, line_to_wref.clone(), false).unwrap();

        assert_eq!(resolver.resolve_by_headword("京都"), Some(line_to_wref[0]));
    }

    #[test]
    fn raw_resolver_resolves_line_refs_in_csv_order() {
        let entries = vec![
            TestEntry {
                headword: "京都",
                reading: "キョウト",
                pos_id: 0,
                reference_id: None,
            },
            TestEntry {
                headword: "東京",
                reading: "トウキョウ",
                pos_id: 1,
                reference_id: None,
            },
            TestEntry {
                headword: "大阪",
                reading: "オオサカ",
                pos_id: 2,
                reference_id: None,
            },
        ];
        let line_to_wref = vec![
            DicWordRef::new(true, 11),
            DicWordRef::new(true, 27),
            DicWordRef::new(true, 42),
        ];
        let resolver = RawDictResolver::new(&entries, line_to_wref.clone(), false).unwrap();

        assert_eq!(
            resolver.resolve_by_line_ref(DicWordRef::new(true, 0)),
            Some(line_to_wref[0])
        );
        assert_eq!(
            resolver.resolve_by_line_ref(DicWordRef::new(true, 1)),
            Some(line_to_wref[1])
        );
        assert_eq!(
            resolver.resolve_by_line_ref(DicWordRef::new(true, 2)),
            Some(line_to_wref[2])
        );
    }

    #[test]
    fn raw_resolver_resolves_reference_id_directly() {
        let entries = vec![
            TestEntry {
                headword: "京都",
                reading: "キョウト",
                pos_id: 0,
                reference_id: Some("kyoto-1"),
            },
            TestEntry {
                headword: "京都",
                reading: "キョウト",
                pos_id: 0,
                reference_id: Some("kyoto-2"),
            },
        ];
        let line_to_wref = vec![DicWordRef::new(true, 11), DicWordRef::new(true, 27)];
        let resolver = RawDictResolver::new(&entries, line_to_wref.clone(), false).unwrap();

        assert_eq!(
            resolver.resolve_entry_key("京都", 0, Some("キョウト"), Some("kyoto-2")),
            Some(line_to_wref[1])
        );
    }

    #[test]
    fn raw_resolver_rejects_duplicate_reference_ids() {
        let entries = vec![
            TestEntry {
                headword: "京都",
                reading: "キョウト",
                pos_id: 0,
                reference_id: Some("kyoto"),
            },
            TestEntry {
                headword: "東京",
                reading: "トウキョウ",
                pos_id: 1,
                reference_id: Some("kyoto"),
            },
        ];

        assert!(RawDictResolver::new(
            &entries,
            vec![DicWordRef::new(true, 1), DicWordRef::new(true, 2)],
            false
        )
        .is_err());
    }
}
