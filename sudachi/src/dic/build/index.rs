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

use crate::dic::build::error::DicWriteError;
use crate::dic::build::primitives::write_u32_array;
use crate::dic::word_id::WordId;
use crate::error::{SudachiError, SudachiResult};
use std::collections::HashMap;
use std::io::Write;

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
    data: HashMap<&'a str, IndexEntry>,
}

impl<'a> IndexBuilder<'a> {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn add(&mut self, key: &'a str, id: WordId) {
        self.data.entry(key).or_default().ids.push(id)
    }

    pub fn build_word_id_table(&mut self) -> SudachiResult<Vec<u8>> {
        // by default assume that there will be 3 entries on average
        let mut result = Vec::with_capacity(self.data.len() * 13);
        for (k, entry) in self.data.iter_mut() {
            entry.offset = result.len();
            // clear stored ids memory after use
            let ids = std::mem::take(&mut entry.ids);
            write_u32_array(&mut result, &ids).map_err(|e| {
                SudachiError::DictionaryCompilationError(DicWriteError {
                    cause: e,
                    line: 0,
                    file: format!("<word id table for `{}` has too much entries>", k),
                })
            })?;
        }
        Ok(result)
    }

    pub fn build_trie(&mut self) -> SudachiResult<Vec<u8>> {
        let mut trie_entries: Vec<(&str, u32)> = Vec::new();
        for (k, v) in self.data.drain() {
            if v.offset > u32::MAX as _ {
                return Err(todo!());
            }
            trie_entries.push((k, v.offset as u32));
        }

        trie_entries.sort_by(|(a, _), (b, _)| a.cmp(b));

        let trie = yada::builder::DoubleArrayBuilder::build(&trie_entries);
        match trie {
            Some(t) => Ok(t),
            None => Err(SudachiError::MissingDictionaryTrie),
        }
    }
}
