use std::fmt;

use crate::dic::grammar::Grammar;
use crate::dic::lexicon::word_infos::WordInfo;
use crate::dic::lexicon::Lexicon;
use crate::lattice::node::Node;
use crate::prelude::*;
use crate::utf8inputtext::Utf8InputText;

/// A morpheme (basic semantic unit of language)
pub struct Morpheme<'a> {
    surface: String,
    pub word_info: WordInfo,
    pub is_oov: bool,
    grammar: &'a Grammar<'a>,
}

impl<'a> Morpheme<'a> {
    /// Create a new `Morpheme`
    pub fn new(
        node: &Node,
        input: &Utf8InputText,
        grammar: &'a Grammar<'a>,
        lexicon: &Lexicon,
    ) -> SudachiResult<Morpheme<'a>> {
        let surface = input.get_original_substring(node.begin..node.end);
        let word_info = match node.word_info.clone() {
            Some(wi) => wi,
            None => {
                let word_id = node.word_id.ok_or(SudachiError::MissingWordId)?;
                lexicon.get_word_info(word_id as usize)?
            }
        };
        let is_oov = node.is_oov;

        Ok(Morpheme {
            surface,
            word_info,
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
