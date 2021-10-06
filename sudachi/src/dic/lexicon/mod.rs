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

use nom::{bytes::complete::take, number::complete::le_u32};
use std::cmp;

use self::trie::Trie;
use self::word_id_table::WordIdTable;
use self::word_infos::{WordInfo, WordInfos};
use self::word_params::WordParams;
use crate::error::SudachiNomResult;
use crate::prelude::*;

pub mod trie;
pub mod word_id_table;
pub mod word_infos;
pub mod word_params;

/// Dictionary lexicon
///
/// Contains trie, word_id, word_param, word_info
pub struct Lexicon<'a> {
    trie: Trie,
    word_id_table: WordIdTable<'a>,
    word_params: WordParams<'a>,
    word_infos: WordInfos<'a>,
}

impl<'a> Lexicon<'a> {
    const USER_DICT_COST_PER_MORPH: i32 = -20;

    pub fn new(
        buf: &[u8],
        original_offset: usize,
        has_synonym_group_ids: bool,
    ) -> SudachiResult<Lexicon> {
        let mut offset = original_offset;

        let (_rest, trie_size) = u32_parser(buf, offset)?;
        offset += 4;
        let (_rest, trie_array) = trie_array_parser(buf, offset, trie_size)?;
        let trie = Trie::new(trie_array, trie_size);
        offset += trie.total_size();

        let (_rest, word_id_table_size) = u32_parser(buf, offset)?;
        let word_id_table = WordIdTable::new(buf, word_id_table_size, offset + 4);
        offset += word_id_table.storage_size();

        let (_rest, word_params_size) = u32_parser(buf, offset)?;
        let word_params = WordParams::new(buf, word_params_size, offset + 4);
        offset += word_params.storage_size();

        let word_infos = WordInfos::new(buf, offset, word_params.size(), has_synonym_group_ids);

        Ok(Lexicon {
            trie,
            word_id_table,
            word_params,
            word_infos,
        })
    }

    /// Returns a list of word_id and length of words that matches given input
    pub fn lookup(&self, input: &[u8], offset: usize) -> SudachiResult<Vec<(u32, usize)>> {
        let result = self.trie.common_prefix_search(input, offset)?;

        let mut l: Vec<(u32, usize)> = Vec::new(); // (word_id, length)
        for item in result {
            let length = item.1;
            for word_id in self.word_id_table.get(item.0)? {
                l.push((word_id, length));
            }
        }

        Ok(l)
    }

    /// Returns word_info for given word_id
    pub fn get_word_info(&self, word_id: u32) -> SudachiResult<WordInfo> {
        self.word_infos.get_word_info(word_id)
    }

    /// Returns word_param for given word_id
    pub fn get_word_param(&self, word_id: u32) -> SudachiResult<(i16, i16, i16)> {
        let left_id = self.word_params.get_left_id(word_id)?;
        let right_id = self.word_params.get_right_id(word_id)?;
        let cost = self.word_params.get_cost(word_id)?;

        Ok((left_id, right_id, cost))
    }

    /// update word_param cost based on current tokenizer
    pub fn update_cost<T: Tokenize>(&mut self, tokenizer: &T) -> SudachiResult<()> {
        for wid in 0..self.word_params.size() as u32 {
            if self.word_params.get_cost(wid)? != std::i16::MIN {
                continue;
            }
            let surface = self.get_word_info(wid)?.surface;
            let ms = tokenizer.tokenize(&surface, Mode::C, false)?;
            let internal_cost = ms.get_internal_cost();
            let cost = internal_cost + Lexicon::USER_DICT_COST_PER_MORPH * ms.len() as i32;
            let cost = cmp::min(cost, std::i16::MAX as i32);
            let cost = cmp::max(cost, std::i16::MIN as i32);
            self.word_params.set_cost(wid, cost as i16);
        }

        Ok(())
    }

    pub fn size(&self) -> u32 {
        self.word_params.size()
    }
}

fn u32_parser(input: &[u8], offset: usize) -> SudachiNomResult<&[u8], u32> {
    nom::sequence::preceded(take(offset), le_u32)(input)
}

fn trie_array_parser(
    input: &[u8],
    offset: usize,
    trie_size: u32,
) -> SudachiNomResult<&[u8], Vec<u32>> {
    // TODO: copied? &[u32] from bytes without copy? Java: `bytes.asIntBuffer();`
    nom::sequence::preceded(take(offset), nom::multi::count(le_u32, trie_size as usize))(input)
}
