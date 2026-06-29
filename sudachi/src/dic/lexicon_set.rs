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

use thiserror::Error;

use crate::dic::binary_loader::BinaryLexicon;
use crate::dic::lexicon::strings::StringPointer;
use crate::dic::lexicon::{Lexicon, LexiconEntry, MAX_DICTIONARIES};
use crate::dic::subset::InfoSubset;
use crate::dic::word_id::{DictId, WordId};
use crate::dic::word_info::{WordInfo, WordInfoEntryIdCursor};
use crate::dic::LexiconAccess;
use crate::prelude::*;

/// Sudachi error
#[derive(Error, Debug, Eq, PartialEq)]
pub enum LexiconSetError {
    #[error("too large word_id {0} in dict {1}")]
    TooLargeWordId(u32, usize),

    #[error("too large dictionary_id {0}")]
    TooLargeDictionaryId(usize),

    #[error("too many user dictionaries")]
    TooManyDictionaries,

    #[error("invalid string pointer of length={0}, offset={1}, alignment={2}")]
    InvalidStringPointer(usize, usize, usize),
}

/// Set of Lexicons
///
/// Handles multiple lexicons as one lexicon
/// The first lexicon in the list must be from system dictionary
pub struct LexiconSet<'a> {
    lexicons: Vec<Lexicon<'a>>,
    pos_offsets: Vec<usize>,
    num_system_pos: usize,
}

#[doc(hidden)]
pub struct WordIdCursor {
    lexicon_index: usize,
    entry_cursor: Option<WordInfoEntryIdCursor>,
}

impl LexiconAccess for LexiconSet<'_> {
    fn lexicon(&self) -> &LexiconSet<'_> {
        self
    }
}

impl<'a> LexiconSet<'a> {
    /// Creates a LexiconSet from a system lexicon
    pub fn from_system_binary(
        system_lexicon: BinaryLexicon<'a>,
        num_system_pos: usize,
    ) -> LexiconSet<'a> {
        let mut lexicon = Lexicon::from_binary(system_lexicon);
        lexicon.set_dic_id(0);
        LexiconSet {
            lexicons: vec![lexicon],
            pos_offsets: vec![0],
            num_system_pos,
        }
    }

    /// Creates a LexiconSet given a system lexicon
    pub fn new(mut system_lexicon: Lexicon<'a>, num_system_pos: usize) -> LexiconSet<'a> {
        system_lexicon.set_dic_id(0);
        LexiconSet {
            lexicons: vec![system_lexicon],
            pos_offsets: vec![0],
            num_system_pos,
        }
    }

    /// Add a lexicon to the lexicon list
    ///
    /// pos_offset: number of pos in the grammar
    pub fn append(
        &mut self,
        mut lexicon: Lexicon<'a>,
        pos_offset: usize,
    ) -> Result<(), LexiconSetError> {
        if self.is_full() {
            return Err(LexiconSetError::TooManyDictionaries);
        }
        lexicon.set_dic_id(self.lexicons.len() as u8);
        self.lexicons.push(lexicon);
        self.pos_offsets.push(pos_offset);
        Ok(())
    }

    /// Returns if dictionary capacity is full
    pub fn is_full(&self) -> bool {
        self.lexicons.len() >= MAX_DICTIONARIES
    }
}

impl LexiconSet<'_> {
    /// Returns iterator which yields all words in the dictionary, starting from the `offset` bytes
    ///
    /// Searches dictionaries in the reverse order: user dictionaries first and then system dictionary
    #[inline]
    pub fn lookup<'b>(
        &'b self,
        input: &'b [u8],
        offset: usize,
    ) -> impl Iterator<Item = LexiconEntry> + 'b {
        // word_id fixup was moved to lexicon itself
        self.lexicons
            .iter()
            .rev()
            .flat_map(move |l| l.lookup(input, offset))
    }

    /// Pipelined + prefetched batch form of [`LexiconSet::lookup`].
    ///
    /// Calls `emit(bucket, entry)` for every match; `bucket` indexes `starts`.
    /// Within a bucket the order matches repeated [`LexiconSet::lookup`] calls
    /// (user dictionaries first, then system, each in trie-walk order), so
    /// grouping by `bucket` reproduces the scalar result.
    #[inline]
    pub fn lookup_batch<F: FnMut(usize, LexiconEntry)>(
        &self,
        input: &[u8],
        starts: &[usize],
        mut emit: F,
    ) {
        // Reverse dictionary order, one lexicon at a time, to match lookup().
        for lexicon in self.lexicons.iter().rev() {
            lexicon.lookup_batch(input, starts, &mut emit);
        }
    }

    /// Checks prefix end offsets in the same dictionary order as lookup(), but
    /// without expanding trie leaves into word IDs.
    #[inline]
    pub(crate) fn check_prefix_ends<F>(
        &self,
        input: &[u8],
        offset: usize,
        mut check: F,
    ) -> Option<bool>
    where
        F: FnMut(usize) -> Option<bool>,
    {
        for lexicon in self.lexicons.iter().rev() {
            for end in lexicon.lookup_prefix_ends(input, offset) {
                if let Some(result) = check(end) {
                    return Some(result);
                }
            }
        }
        None
    }

    /// Returns WordInfo for given WordId
    pub fn get_word_info(&self, id: WordId) -> SudachiResult<WordInfo> {
        self.get_word_info_subset(id, InfoSubset::all())
    }

    /// Returns WordInfo for given WordId.
    /// Only fills a requested subset of fields.
    /// Rest will be of default values (0 or empty).
    pub fn get_word_info_subset(&self, id: WordId, subset: InfoSubset) -> SudachiResult<WordInfo> {
        let dict_id = id.dict();
        let lexicon = self
            .lexicons
            .get(dict_id.as_raw() as usize)
            .ok_or(SudachiError::InvalidWordId(id))?;
        let word_info_data = lexicon.get_word_info(id.entry(), subset)?.resolve(
            dict_id,
            self.num_system_pos,
            &self.pos_offsets,
            subset,
        );

        Ok(WordInfo::new(word_info_data, id))
    }

    /// Returns word_param for given word_id
    pub fn get_word_param(&self, id: WordId) -> (i16, i16, i16) {
        let dict_id = id.dict().as_raw() as usize;
        self.lexicons[dict_id].get_word_param(id.entry())
    }

    /// Returns word_param for given word_id.
    pub fn get_word_param_checked(&self, id: WordId) -> SudachiResult<(i16, i16, i16)> {
        let dict_id = id.dict().as_raw() as usize;
        match self.lexicons.get(dict_id) {
            Some(lexicon) => lexicon
                .get_word_param_checked(id.entry())
                .ok_or(SudachiError::InvalidWordId(id)),
            None => Err(SudachiError::InvalidWordId(id)),
        }
    }

    #[inline]
    pub fn get_string(&self, word_id: WordId, strptr: StringPointer) -> SudachiResult<String> {
        self.lexicons[word_id.dict().as_raw() as usize].get_string(strptr)
    }

    pub fn size(&self) -> u32 {
        self.lexicons.iter().fold(0, |acc, lex| acc + lex.size())
    }

    pub fn word_ids(&self) -> impl Iterator<Item = SudachiResult<WordId>> + '_ {
        self.lexicons.iter().enumerate().flat_map(|(dict_id, lex)| {
            let dict_id = DictId::new(dict_id as u8);
            lex.entry_ids()
                .map(move |entry| entry.map(|entry| WordId::from_parts(dict_id, entry)))
        })
    }

    #[doc(hidden)]
    pub fn word_id_cursor(&self) -> WordIdCursor {
        WordIdCursor {
            lexicon_index: 0,
            entry_cursor: self.lexicons.first().map(Lexicon::entry_id_cursor),
        }
    }

    #[doc(hidden)]
    pub fn next_word_id(&self, cursor: &mut WordIdCursor) -> SudachiResult<Option<WordId>> {
        loop {
            let Some(lexicon) = self.lexicons.get(cursor.lexicon_index) else {
                return Ok(None);
            };
            let Some(entry_cursor) = cursor.entry_cursor.as_mut() else {
                return Ok(None);
            };
            if let Some(entry) = lexicon.next_entry_id(entry_cursor)? {
                let dict_id = DictId::new(cursor.lexicon_index as u8);
                return Ok(Some(WordId::from_parts(dict_id, entry)));
            }

            cursor.lexicon_index += 1;
            cursor.entry_cursor = self
                .lexicons
                .get(cursor.lexicon_index)
                .map(Lexicon::entry_id_cursor);
        }
    }

    pub fn system_word_ids_in_order(&self) -> Vec<WordId> {
        if self.lexicons.is_empty() {
            return Vec::new();
        }
        self.lexicons[0]
            .entry_ids_in_order()
            .into_iter()
            .map(|entry| WordId::from_parts(DictId::SYSTEM, entry))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::dic::binary_loader::LoadedDictionary;

    const TEST_SYSTEM_DIC: &[u8] = include_bytes!("../../tests/resources/system.dic.test");

    #[test]
    fn check_prefix_ends_matches_lookup_end_order() {
        let dictionary = LoadedDictionary::load_system(TEST_SYSTEM_DIC).unwrap();
        let lexicon_set = &dictionary.lexicon_set;
        let inputs = [
            "ばな。なです。",
            "東京都に行く",
            "京都",
            "あいうえお",
            "1.と2.が。",
        ];

        for input in inputs {
            let bytes = input.as_bytes();
            for (offset, _) in input.char_indices() {
                let mut checked_ends = Vec::new();
                let decision = lexicon_set.check_prefix_ends(bytes, offset, |end| {
                    checked_ends.push(end);
                    None::<bool>
                });

                assert_eq!(decision, None);

                let mut expected_ends = Vec::new();
                for lexicon in lexicon_set.lexicons.iter().rev() {
                    let mut lookup_ends = Vec::new();
                    for entry in lexicon.lookup(bytes, offset) {
                        if lookup_ends.last() != Some(&entry.end) {
                            lookup_ends.push(entry.end);
                        }
                    }

                    let prefix_ends = lexicon
                        .lookup_prefix_ends(bytes, offset)
                        .collect::<Vec<_>>();
                    assert_eq!(lookup_ends, prefix_ends);
                    expected_ends.extend(prefix_ends);
                }

                assert_eq!(checked_ends, expected_ends);
            }
        }
    }
}
