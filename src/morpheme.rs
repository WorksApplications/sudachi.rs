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

use std::fmt;

use crate::dic::grammar::Grammar;
use crate::dic::lexicon::word_infos::WordInfo;
use crate::dic::lexicon_set::LexiconSet;
use crate::input_text::utf8_input_text::Utf8InputText;
use crate::lattice::node::Node;
use crate::prelude::*;

/// A morpheme (basic semantic unit of language)
pub struct Morpheme<'a> {
    surface: String,
    pub word_info: WordInfo,
    pub cost: i16,
    pub dictionary_id: i32,
    pub is_oov: bool,
    grammar: &'a Grammar<'a>,
}

impl<'a> Morpheme<'a> {
    /// Create a new `Morpheme`
    pub fn new(
        node: &Node,
        input: &Utf8InputText,
        grammar: &'a Grammar<'a>,
        lexicon: &LexiconSet,
    ) -> SudachiResult<Morpheme<'a>> {
        let surface = input.get_original_substring(node.begin..node.end);
        let word_info = match node.word_info.as_ref() {
            Some(wi) => wi.clone(),
            None => {
                let word_id = node.word_id.ok_or(SudachiError::MissingWordId)?;
                lexicon.get_word_info(word_id)?
            }
        };
        let is_oov = node.is_oov;
        let cost = node.cost;
        let dictionary_id = node.get_dictionary_id();

        Ok(Morpheme {
            surface,
            word_info,
            cost,
            dictionary_id,
            is_oov,
            grammar,
        })
    }

    /// Returns the text of morpheme.
    ///
    /// When the input text is normalized, some morphemes have the same surface.
    pub fn surface(&self) -> &String {
        &self.surface
    }

    /// Part of speech
    pub fn pos(&self) -> SudachiResult<&Vec<String>> {
        let res = &self
            .grammar
            .pos_list
            .get(self.word_info.pos_id as usize)
            .ok_or(SudachiError::MissingPartOfSpeech)?;
        Ok(res)
    }

    /// Normalized form of morpheme
    ///
    /// This method returns the form normalizing inconsistent spellings and
    /// inflected forms.
    pub fn normalized_form(&self) -> &String {
        &self.word_info.normalized_form
    }

    /// Returns the reading form of morpheme.
    ///
    /// Returns Japanese syllabaries 'フリガナ' in katakana.
    pub fn reading_form(&self) -> &String {
        &self.word_info.reading_form
    }

    /// Returns the dictionary form of morpheme.
    ///
    /// "Dictionary form" means a word's lemma and "終止形" in Japanese.
    pub fn dictionary_form(&self) -> &String {
        &self.word_info.dictionary_form
    }
}

impl<'a> fmt::Debug for Morpheme<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Morpheme")
            .field("surface", self.surface())
            .field("pos", &self.pos())
            .field("normalized_form", self.normalized_form())
            .field("reading_form", self.reading_form())
            .field("dictionary_form", self.dictionary_form())
            .finish()
    }
}
