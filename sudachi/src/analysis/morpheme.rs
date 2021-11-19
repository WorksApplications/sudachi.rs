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

use crate::analysis::node::{LatticeNode, PathCost, ResultNode};
use crate::analysis::stateful_tokenizer::StatefulTokenizer;
use crate::analysis::stateless_tokenizer::DictionaryAccess;
use crate::dic::grammar::Grammar;
use crate::dic::lexicon::word_infos::WordInfo;
use crate::dic::subset::InfoSubset;
use crate::dic::word_id::WordId;
use crate::prelude::*;

/// A list of morphemes
pub struct MorphemeList<T> {
    dict: T,
    input_text: String,
    path: Vec<ResultNode>,
    subset: InfoSubset,
}

impl<T: DictionaryAccess> MorphemeList<T> {
    pub fn from_components(
        dict: T,
        original: String,
        path: Vec<ResultNode>,
        subset: InfoSubset,
    ) -> SudachiResult<Self> {
        let list = Self {
            dict,
            input_text: original,
            path,
            subset,
        };
        Ok(list)
    }

    /// Returns an empty morpheme list.
    pub fn empty(dict: T) -> Self {
        Self {
            dict,
            input_text: String::new(),
            path: Vec::new(),
            subset: InfoSubset::default(),
        }
    }

    pub fn collect_results<U: DictionaryAccess>(
        &mut self,
        analyzer: &mut StatefulTokenizer<U>,
    ) -> SudachiResult<()> {
        analyzer.swap_result(&mut self.input_text, &mut self.path, &mut self.subset);
        Ok(())
    }

    pub fn get_grammar(&self) -> &Grammar {
        self.dict.grammar()
    }
}

impl<T: DictionaryAccess + Clone> MorphemeList<T> {
    /// Returns a new morpheme list splitting the morpheme with a given mode.
    pub fn split(&self, mode: Mode, index: usize) -> SudachiResult<MorphemeList<T>> {
        let node = &self.path[index];
        let num_splits = node.num_splits(mode);

        let path = if num_splits <= 1 {
            vec![node.clone()]
        } else {
            node.split(mode, self.dict.lexicon(), self.subset).collect()
        };

        Ok(MorphemeList {
            dict: self.dict.clone(),
            input_text: self.input_text.clone(),
            path,
            subset: self.subset,
        })
    }
}

impl<T> MorphemeList<T> {
    pub fn iter(&self) -> MorphemeIter<T> {
        MorphemeIter {
            list: &self,
            index: 0,
        }
    }

    pub fn surface(&self) -> &str {
        if self.len() == 0 {
            return &self.input_text;
        }

        // can be a slice of another list
        let begin = self.path.first().unwrap().begin_bytes();
        let end = self.path.last().unwrap().end_bytes();
        &self.input_text[begin..end]
    }

    pub fn get_node(&self, index: usize) -> &ResultNode {
        &self.path[index]
    }

    pub fn get_begin(&self, index: usize) -> usize {
        self.path[index].begin_bytes()
    }

    pub fn get_end(&self, index: usize) -> usize {
        self.path[index].end_bytes()
    }

    /// Returns a substring of the original text which corresponds to the morpheme
    pub fn get_surface(&self, index: usize) -> &str {
        let node = &self.path[index];
        &self.input_text[node.begin_bytes()..node.end_bytes()]
    }

    pub fn get_word_info(&self, index: usize) -> &WordInfo {
        self.path[index].word_info()
    }

    pub fn is_oov(&self, index: usize) -> bool {
        self.path[index].word_id().is_oov()
    }

    /// Returns the total cost of the path
    pub fn get_internal_cost(&self) -> i32 {
        if self.len() == 0 {
            return 0;
        }

        let first = &self.path[0];
        let last = self.path.last().unwrap();
        last.total_cost() - first.total_cost()
    }

    pub fn len(&self) -> usize {
        self.path.len()
    }

    pub fn get(&self, index: usize) -> Morpheme<T> {
        debug_assert!(index < self.path.len());
        Morpheme { list: self, index }
    }
}

/// Iterates over morpheme list
pub struct MorphemeIter<'a, T> {
    list: &'a MorphemeList<T>,
    index: usize,
}

impl<'a, T: DictionaryAccess> Iterator for MorphemeIter<'a, T> {
    type Item = Morpheme<'a, T>;

    fn next(&mut self) -> Option<Self::Item> {
        if let None = self.list.path.get(self.index) {
            return None;
        }

        let morpheme = Morpheme {
            list: self.list,
            index: self.index,
        };

        self.index += 1;
        Some(morpheme)
    }
}

/// A morpheme (basic semantic unit of language)
pub struct Morpheme<'a, T> {
    list: &'a MorphemeList<T>,
    index: usize,
}

impl<T: DictionaryAccess> Morpheme<'_, T> {
    /// Returns the part of speech
    pub fn part_of_speech(&self) -> SudachiResult<&[String]> {
        let pos_id = self.part_of_speech_id();
        let pos = self
            .list
            .get_grammar()
            .pos_list
            .get(pos_id as usize)
            .ok_or(SudachiError::MissingPartOfSpeech)?;
        Ok(pos)
    }
}

impl<T: DictionaryAccess + Clone> Morpheme<'_, T> {
    /// Returns new morpheme list splitting the morpheme with given mode.
    pub fn split(&self, mode: Mode) -> SudachiResult<MorphemeList<T>> {
        self.list.split(mode, self.index)
    }
}

impl<T> Morpheme<'_, T> {
    /// Returns the begin index of morpheme in the original text
    pub fn begin(&self) -> usize {
        self.list.get_begin(self.index)
    }

    /// Returns the end index of morpehme in the original text
    pub fn end(&self) -> usize {
        self.list.get_end(self.index)
    }

    /// Returns a substring of the original text which corresponds to the morpheme
    pub fn surface(&self) -> &str {
        self.list.get_surface(self.index)
    }

    pub fn part_of_speech_id(&self) -> u16 {
        self.get_word_info().pos_id()
    }

    /// Returns the dictionary form of morpheme
    ///
    /// "Dictionary form" means a word's lemma and "終止形" in Japanese.
    pub fn dictionary_form(&self) -> &str {
        &self.get_word_info().dictionary_form()
    }

    /// Returns the normalized form of morpheme
    ///
    /// This method returns the form normalizing inconsistent spellings and inflected forms
    pub fn normalized_form(&self) -> &str {
        &self.get_word_info().normalized_form()
    }

    /// Returns the reading form of morpheme.
    ///
    /// Returns Japanese syllabaries 'フリガナ' in katakana.
    pub fn reading_form(&self) -> &str {
        &self.get_word_info().reading_form()
    }

    /// Returns if this morpheme is out of vocabulary
    pub fn is_oov(&self) -> bool {
        self.list.is_oov(self.index)
    }

    /// Returns the word id of morpheme
    pub fn word_id(&self) -> WordId {
        self.list.get_node(self.index).word_id()
    }

    /// Returns the dictionary id where the morpheme belongs
    ///
    /// Returns -1 if the morpheme is oov
    pub fn dictionary_id(&self) -> i32 {
        let wid = self.word_id();
        if wid.is_oov() {
            -1
        } else {
            wid.dic() as i32
        }
    }

    pub fn synonym_group_ids(&self) -> &[u32] {
        &self.get_word_info().synonym_group_ids()
    }

    pub fn get_word_info(&self) -> &WordInfo {
        self.list.get_word_info(self.index)
    }

    pub fn index(&self) -> usize {
        self.index
    }
}

impl<T: DictionaryAccess> std::fmt::Debug for Morpheme<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Morpheme")
            .field("surface", &self.surface())
            .field("pos", &self.part_of_speech())
            .field("normalized_form", &self.normalized_form())
            .field("reading_form", &self.reading_form())
            .field("dictionary_form", &self.dictionary_form())
            .finish()
    }
}
