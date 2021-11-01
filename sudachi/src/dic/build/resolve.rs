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
use crate::dic::build::error::{DicWriteReason, DicWriteResult};
use crate::dic::build::lexicon::{RawLexiconEntry, SplitUnit, SplitUnitResolver};
use crate::dic::word_id::WordId;
use crate::error::SudachiResult;
use std::collections::HashMap;

pub struct BuiltDictResolver<D: DictionaryAccess> {
    dict: D,
}

impl<D: DictionaryAccess> BuiltDictResolver<D> {
    pub fn new(dict: D) -> Self {
        Self { dict }
    }
}

impl<D: DictionaryAccess> SplitUnitResolver for BuiltDictResolver<D> {
    fn resolve_inline(&self, surface: &str, pos: u16, reading: Option<&str>) -> Option<WordId> {
        let len = surface.len();
        let ids = self
            .dict
            .lexicon()
            .lookup(surface.as_bytes(), 0)
            .filter(|e| e.end == len)
            .map(|e| e.word_id);
        for wid in ids {
            let winfo = match self.dict.lexicon().get_word_info(wid) {
                Ok(wi) => wi,
                Err(_) => return None,
            };
            if winfo.pos_id == pos && reading.as_deref().unwrap_or(&surface) == winfo.reading_form {
                return Some(wid);
            }
        }
        None
    }
}

pub struct RawDictResolver<'a> {
    data: HashMap<&'a str, Vec<(u16, Option<&'a str>, WordId)>>,
}

impl<'a> RawDictResolver<'a> {
    pub(crate) fn new(entries: &'a [RawLexiconEntry], user: bool) -> Self {
        let mut data: HashMap<&'a str, Vec<(u16, Option<&'a str>, WordId)>> = HashMap::new();

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
