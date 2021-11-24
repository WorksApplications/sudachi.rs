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

use std::cmp;
use std::mem::size_of;

use crate::analysis::stateful_tokenizer::StatefulTokenizer;
use crate::analysis::stateless_tokenizer::DictionaryAccess;
use crate::dic::subset::InfoSubset;
use crate::dic::word_id::WordId;
use nom::{bytes::complete::take, number::complete::le_u32};

use crate::error::SudachiNomResult;
use crate::prelude::*;

use self::trie::Trie;
use self::word_id_table::WordIdTable;
use self::word_infos::{WordInfo, WordInfos};
use self::word_params::WordParams;

pub mod trie;
pub mod word_id_table;
pub mod word_infos;
pub mod word_params;

/// The first 4 bits of word_id are used to indicate that from which lexicon
/// the word comes, thus we can only hold 15 lexicons in the same time.
/// 16th is reserved for marking OOVs.
pub const MAX_DICTIONARIES: usize = 15;

/// Dictionary lexicon
///
/// Contains trie, word_id, word_param, word_info
pub struct Lexicon<'a> {
    trie: Trie<'a>,
    word_id_table: WordIdTable<'a>,
    word_params: WordParams<'a>,
    word_infos: WordInfos<'a>,
    lex_id: u8,
}

/// Result of the Lexicon lookup
#[derive(Eq, PartialEq, Debug)]
pub struct LexiconEntry {
    /// Id of the returned word
    pub word_id: WordId,
    /// Byte index of the word end
    pub end: usize,
}

impl LexiconEntry {
    pub fn new(word_id: WordId, end: usize) -> LexiconEntry {
        LexiconEntry { word_id, end }
    }
}

impl<'a> Lexicon<'a> {
    const USER_DICT_COST_PER_MORPH: i32 = -20;

    pub fn parse(
        buf: &[u8],
        original_offset: usize,
        has_synonym_group_ids: bool,
    ) -> SudachiResult<Lexicon> {
        let mut offset = original_offset;

        let (_rest, trie_size) = u32_parser_offset(buf, offset)?;
        offset += 4;
        let trie_array = trie_array_parser(buf, offset, trie_size)?;
        let trie = Trie::new(trie_array, trie_size as usize);
        offset += trie.total_size();

        let (_rest, word_id_table_size) = u32_parser_offset(buf, offset)?;
        let word_id_table = WordIdTable::new(buf, word_id_table_size, offset + 4);
        offset += word_id_table.storage_size();

        let (_rest, word_params_size) = u32_parser_offset(buf, offset)?;
        let word_params = WordParams::new(buf, word_params_size, offset + 4);
        offset += word_params.storage_size();

        let word_infos = WordInfos::new(buf, offset, word_params.size(), has_synonym_group_ids);

        Ok(Lexicon {
            trie,
            word_id_table,
            word_params,
            word_infos,
            lex_id: u8::MAX,
        })
    }

    /// Assign lexicon id to the current Lexicon
    pub fn set_dic_id(&mut self, id: u8) {
        assert!(id < MAX_DICTIONARIES as u8);
        self.lex_id = id
    }

    #[inline]
    fn word_id(&self, raw_id: u32) -> WordId {
        return WordId::new(self.lex_id, raw_id);
    }

    /// Returns an iterator of word_id and end of words that matches given input
    #[inline]
    pub fn lookup(
        &'a self,
        input: &'a [u8],
        offset: usize,
    ) -> impl Iterator<Item = LexiconEntry> + 'a {
        debug_assert!(self.lex_id < MAX_DICTIONARIES as u8);
        self.trie
            .common_prefix_iterator(input, offset)
            .flat_map(move |e| {
                self.word_id_table
                    .entries(e.value as usize)
                    .map(move |wid| LexiconEntry::new(self.word_id(wid), e.end))
            })
    }

    /// Returns WordInfo for given word_id
    ///
    /// WordInfo will contain only fields included in InfoSubset
    pub fn get_word_info(&self, word_id: u32, subset: InfoSubset) -> SudachiResult<WordInfo> {
        self.word_infos.get_word_info(word_id, subset)
    }

    /// Returns word_param for given word_id.
    /// Params are (left_id, right_id, cost).
    #[inline]
    pub fn get_word_param(&self, word_id: u32) -> (i16, i16, i16) {
        self.word_params.get_params(word_id)
    }

    /// update word_param cost based on current tokenizer
    pub fn update_cost<D: DictionaryAccess>(&mut self, dict: &D) -> SudachiResult<()> {
        let mut tok = StatefulTokenizer::create(dict, false, Mode::C);
        let mut ms = MorphemeList::empty(dict);
        for wid in 0..self.word_params.size() {
            if self.word_params.get_cost(wid) != i16::MIN {
                continue;
            }
            let wi = self.get_word_info(wid, InfoSubset::SURFACE)?;
            tok.reset().push_str(wi.surface());
            tok.do_tokenize()?;
            ms.collect_results(&mut tok)?;
            let internal_cost = ms.get_internal_cost();
            let cost = internal_cost + Lexicon::USER_DICT_COST_PER_MORPH * ms.len() as i32;
            let cost = cmp::min(cost, i16::MAX as i32);
            let cost = cmp::max(cost, i16::MIN as i32);
            self.word_params.set_cost(wid, cost as i16);
        }

        Ok(())
    }

    pub fn size(&self) -> u32 {
        self.word_params.size()
    }
}

fn u32_parser_offset(input: &[u8], offset: usize) -> SudachiNomResult<&[u8], u32> {
    nom::sequence::preceded(take(offset), le_u32)(input)
}

fn trie_array_parser(input: &[u8], offset: usize, trie_size: u32) -> SudachiResult<&[u8]> {
    let trie_start = offset;
    let trie_end = offset + (trie_size as usize) * size_of::<u32>();
    if input.len() < trie_start {
        return Err(SudachiError::InvalidRange(trie_start, trie_end));
    }
    if input.len() < trie_end {
        return Err(SudachiError::InvalidRange(trie_start, trie_end));
    }
    let trie_data = &input[trie_start..trie_end];
    Ok(trie_data)
}
