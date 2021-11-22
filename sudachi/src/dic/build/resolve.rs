/*
 *  Copyright (c) 2021 Works Applications Co., Ltd.
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

use crate::analysis::stateless_tokenizer::DictionaryAccess;
use crate::dic::build::lexicon::{RawLexiconEntry, SplitUnitResolver};
use crate::dic::lexicon::word_infos::WordInfoData;
use crate::dic::subset::InfoSubset;
use crate::dic::word_id::WordId;
use crate::error::SudachiResult;
use crate::util::fxhash::FxBuildHasher;
use std::collections::HashMap;

/// We can't use trie to resolve splits because it is possible that refs are not in trie
/// This resolver has to be owning because the dictionary content is lazily loaded and transient
pub struct BinDictResolver {
    index: HashMap<String, Vec<(u16, Option<String>, WordId)>, FxBuildHasher>,
}

impl BinDictResolver {
    pub fn new<D: DictionaryAccess>(dict: D) -> SudachiResult<Self> {
        let lex = dict.lexicon();
        let size = lex.size();
        let mut index: HashMap<String, Vec<(u16, Option<String>, WordId)>, FxBuildHasher> =
            HashMap::default();
        for id in 0..size {
            let wid = WordId::new(0, id);
            let winfo: WordInfoData = lex
                .get_word_info_subset(
                    wid,
                    InfoSubset::SURFACE | InfoSubset::READING_FORM | InfoSubset::POS_ID,
                )?
                .into();
            let surface = winfo.surface;
            let reading = winfo.reading_form;
            let pos_id = winfo.pos_id;

            let rdfield = if reading.is_empty() || surface == reading {
                None
            } else {
                Some(reading)
            };

            index
                .entry(surface)
                .or_default()
                .push((pos_id, rdfield, wid));
        }

        Ok(Self { index })
    }
}

impl SplitUnitResolver for BinDictResolver {
    fn resolve_inline(&self, surface: &str, pos: u16, reading: Option<&str>) -> Option<WordId> {
        self.index.get(surface).and_then(|v| {
            for (p, rd, wid) in v {
                if *p == pos && reading.eq(&rd.as_deref()) {
                    return Some(*wid);
                }
            }
            None
        })
    }
}

pub struct RawDictResolver<'a> {
    data: HashMap<&'a str, Vec<(u16, Option<&'a str>, WordId)>, FxBuildHasher>,
}

impl<'a> RawDictResolver<'a> {
    pub(crate) fn new(entries: &'a [RawLexiconEntry], user: bool) -> Self {
        let mut data: HashMap<&'a str, Vec<(u16, Option<&'a str>, WordId)>, FxBuildHasher> =
            HashMap::default();

        let dic_id = if user { 1 } else { 0 };

        for (i, e) in entries.iter().enumerate() {
            let surface: &'a str = e.surface();
            let reading: &'a str = e.reading();
            let wid = WordId::new(dic_id, i as u32);

            let read_opt = if surface == reading {
                None
            } else {
                Some(reading)
            };

            data.entry(surface)
                .or_default()
                .push((e.pos, read_opt, wid));
        }

        Self { data }
    }
}

impl SplitUnitResolver for RawDictResolver<'_> {
    fn resolve_inline(&self, surface: &str, pos: u16, reading: Option<&str>) -> Option<WordId> {
        self.data.get(surface).and_then(|data| {
            for (p, rd, wid) in data {
                if *p == pos && *rd == reading {
                    return Some(*wid);
                }
            }
            None
        })
    }
}

pub(crate) struct ChainedResolver<A, B> {
    a: A,
    b: B,
}

impl<A: SplitUnitResolver, B: SplitUnitResolver> ChainedResolver<A, B> {
    pub(crate) fn new(a: A, b: B) -> Self {
        Self { a, b }
    }
}

impl<A: SplitUnitResolver, B: SplitUnitResolver> SplitUnitResolver for ChainedResolver<A, B> {
    fn resolve_inline(&self, surface: &str, pos: u16, reading: Option<&str>) -> Option<WordId> {
        self.a
            .resolve_inline(surface, pos, reading)
            .or_else(|| self.b.resolve_inline(surface, pos, reading))
    }
}
