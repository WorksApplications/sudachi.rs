/*
 * Copyright (c) 2021 Works Applications Co., Ltd.
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

use crate::prelude::*;

pub struct Trie {
    array: Vec<u32>,
    size: u32, // number of elements
}

impl Trie {
    pub fn new(array: Vec<u32>, size: u32) -> Trie {
        Trie { array, size }
    }

    pub fn total_size(&self) -> usize {
        4 * self.size as usize
    }

    pub fn common_prefix_search(
        &self,
        input: &[u8],
        offset: usize,
    ) -> SudachiResult<Vec<(usize, usize)>> {
        let mut result = Vec::new();

        let mut node_pos: usize = 0;
        let mut unit: usize = *self
            .array
            .get(node_pos)
            .ok_or(SudachiError::MissingDictionaryTrie)? as usize;
        node_pos ^= Trie::offset(unit);

        for i in offset..input.len() {
            let k = input.get(i).ok_or(SudachiError::MissingDictionaryTrie)?;
            node_pos ^= *k as usize;
            unit = *self
                .array
                .get(node_pos)
                .ok_or(SudachiError::MissingDictionaryTrie)? as usize;
            if Trie::label(unit) != *k as usize {
                return Ok(result);
            }

            node_pos ^= Trie::offset(unit);
            if Trie::has_leaf(unit) {
                let r = (
                    Trie::value(
                        *self
                            .array
                            .get(node_pos)
                            .ok_or(SudachiError::MissingDictionaryTrie)?
                            as usize,
                    ),
                    i + 1,
                );
                result.push(r);
            }
        }

        Ok(result)
    }

    fn has_leaf(unit: usize) -> bool {
        ((unit >> 8) & 1) == 1
    }

    fn value(unit: usize) -> usize {
        unit & ((1 << 31) - 1)
    }

    fn label(unit: usize) -> usize {
        unit & ((1 << 31) | 0xFF)
    }

    fn offset(unit: usize) -> usize {
        (unit >> 10) << ((unit & (1 << 9)) >> 6)
    }
}
