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
use crate::analysis::stateless_tokenizer::DictionaryAccess;
use crate::dic::lexicon::word_infos::WordInfo;
use crate::dic::word_id::WordId;
use crate::input_text::InputTextIndex;
use crate::prelude::*;
use std::cell::Ref;

/// A morpheme (basic semantic unit of language)
pub struct Morpheme<'a, T> {
    list: &'a MorphemeList<T>,
    index: usize,
}

impl<T: DictionaryAccess> Morpheme<'_, T> {
    /// Returns the part of speech
    pub fn part_of_speech(&self) -> &[String] {
        self.list
            .dict()
            .grammar()
            .pos_components(self.part_of_speech_id())
    }
}

impl<T: DictionaryAccess + Clone> Morpheme<'_, T> {
    /// Returns new morpheme list splitting the morpheme with given mode.
    #[deprecated(note = "use split_into", since = "0.6.1")]
    pub fn split(&self, mode: Mode) -> SudachiResult<MorphemeList<T>> {
        #[allow(deprecated)]
        self.list.split(mode, self.index)
    }
}

impl<'a, T: DictionaryAccess> Morpheme<'a, T> {
    pub(crate) fn for_list(list: &'a MorphemeList<T>, index: usize) -> Self {
        Morpheme { list, index }
    }

    #[inline]
    pub(crate) fn node(&self) -> &ResultNode {
        self.list.node(self.index)
    }

    /// Returns the begin index in bytes of the morpheme in the original text
    pub fn begin(&self) -> usize {
        self.list.input().to_orig_byte_idx(self.node().begin())
    }

    /// Returns the end index in bytes of the morpheme in the original text
    pub fn end(&self) -> usize {
        self.list.input().to_orig_byte_idx(self.node().end())
    }

    /// Returns the codepoint offset of the morpheme begin in the original text
    pub fn begin_c(&self) -> usize {
        self.list.input().to_orig_char_idx(self.node().begin())
    }

    /// Returns the codepoint offset of the morpheme begin in the original text
    pub fn end_c(&self) -> usize {
        self.list.input().to_orig_char_idx(self.node().end())
    }

    /// Returns a substring of the original text which corresponds to the morpheme
    pub fn surface(&self) -> Ref<str> {
        let inp = self.list.input();
        Ref::map(inp, |i| i.orig_slice(self.node().bytes_range()))
    }

    pub fn part_of_speech_id(&self) -> u16 {
        self.node().word_info().pos_id()
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
        self.word_id().is_oov()
    }

    /// Returns the word id of morpheme
    pub fn word_id(&self) -> WordId {
        self.node().word_id()
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
        self.node().word_info()
    }

    /// Returns the index of this morpheme
    pub fn index(&self) -> usize {
        self.index
    }

    /// Splits morpheme and writes sub-morphemes into the provided list.
    /// The resulting list is _not_ cleared before that.
    /// Returns true if split has produced any elements.
    pub fn split_into(&self, mode: Mode, out: &mut MorphemeList<T>) -> SudachiResult<bool> {
        self.list.split_into(mode, self.index, out)
    }

    /// Returns total cost from the beginning of the path
    pub fn total_cost(&self) -> i32 {
        return self.node().total_cost();
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
