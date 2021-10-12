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

use thiserror::Error;

use crate::dic::lexicon::word_infos::WordInfo;
use crate::dic::lexicon::{Lexicon, LexiconEntry, MAX_DICTIONARIES};
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
}

/// Set of Lexicons
///
/// Handles multiple lexicons as one lexicon
/// The first lexicon in the list must be from system dictionary
pub struct LexiconSet<'a> {
    lexicons: Vec<Lexicon<'a>>,
    pos_offsets: Vec<usize>,
}

impl<'a> LexiconSet<'a> {
    /// Creates a LexiconSet given a lexicon
    ///
    /// It is assumed that the passed lexicon is the system dictionary
    pub fn new(mut system_lexicon: Lexicon) -> LexiconSet {
        system_lexicon.set_dic_id(0);
        LexiconSet {
            lexicons: vec![system_lexicon],
            pos_offsets: vec![0],
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

    /// Returns word_info for given word_id
    pub fn get_word_info(&self, dictword_id: u32) -> SudachiResult<WordInfo> {
        let (dict_id, word_id) = LexiconSet::decode_dictword_id(dictword_id);
        let mut word_info = self.lexicons[dict_id].get_word_info(word_id)?;
        let pos_id = word_info.pos_id;
        if dict_id > 0 && pos_id as usize >= self.pos_offsets[1] {
            // user defined part-of-speech
            word_info.pos_id =
                (pos_id as usize - self.pos_offsets[1] + self.pos_offsets[dict_id]) as u16;
        }
        self.update_dict_id(&mut word_info.a_unit_split, dict_id)?;
        self.update_dict_id(&mut word_info.b_unit_split, dict_id)?;
        self.update_dict_id(&mut word_info.word_structure, dict_id)?;

        Ok(word_info)
    }

    /// Returns word_param for given word_id
    pub fn get_word_param(&self, dictword_id: u32) -> SudachiResult<(i16, i16, i16)> {
        let (dict_id, word_id) = LexiconSet::decode_dictword_id(dictword_id);
        self.lexicons[dict_id].get_word_param(word_id)
    }

    /// Merge dict_id and word_id into one u32
    ///
    /// We use top 4 bits for dict_id
    fn build_dictword_id(&self, dict_id: usize, word_id: u32) -> Result<u32, LexiconSetError> {
        if word_id > 0x0FFFFFFF {
            return Err(LexiconSetError::TooLargeWordId(word_id, dict_id));
        }
        if dict_id > self.lexicons.len() {
            return Err(LexiconSetError::TooLargeDictionaryId(dict_id));
        }
        Ok((dict_id as u32) << 28 | word_id)
    }
    pub fn get_dictionary_id(dictword_id: u32) -> usize {
        (dictword_id >> 28) as usize
    }
    fn get_word_id(dictword_id: u32) -> u32 {
        dictword_id & 0x0FFFFFFF
    }
    fn decode_dictword_id(dictword_id: u32) -> (usize, u32) {
        let dict_id = LexiconSet::get_dictionary_id(dictword_id);
        let word_id = LexiconSet::get_word_id(dictword_id);
        (dict_id, word_id)
    }

    fn update_dict_id(&self, split: &mut Vec<u32>, dict_id: usize) -> SudachiResult<()> {
        for i in 0..split.len() {
            let (crr_dict_id, word_id) = LexiconSet::decode_dictword_id(split[i]);
            if crr_dict_id > 0 {
                // update if target word is not in system_dict
                split[i] = self.build_dictword_id(dict_id, word_id)?;
            }
        }
        Ok(())
    }

    pub fn size(&self) -> u32 {
        self.lexicons.iter().fold(0, |acc, lex| acc + lex.size())
    }
}
