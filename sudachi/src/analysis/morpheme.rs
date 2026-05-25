/*
 *  Copyright (c) 2021-2026 Works Applications Co., Ltd.
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

use crate::analysis::mlist::MorphemeList;
use crate::analysis::node::{LatticeNode, PathCost, ResultNode};
use crate::analysis::Mode;
use crate::dic::subset::InfoSubset;
use crate::dic::word_id::WordId;
use crate::dic::word_info::{WordInfo, WordInfoData, WordInfoResolver};
use crate::dic::{DictionaryAccess, LexiconAccess};
use crate::error::{SudachiError, SudachiResult};
use crate::input_text::InputTextIndex;
use std::borrow::Cow;
use std::cell::Ref;
use std::fmt::{Debug, Display, Formatter};
use std::ops::Deref;

/// Surface text returned by a morpheme.
///
/// Analysis-result morphemes borrow their surface from an `InputBuffer`, while
/// standalone morphemes expose a dictionary headword. This wrapper keeps both
/// cases usable through the same `Deref<Target = str>` interface.
pub enum MorphemeSurface<'a> {
    Input(Ref<'a, str>),
    Headword(&'a str),
}

impl Deref for MorphemeSurface<'_> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        match self {
            MorphemeSurface::Input(surface) => surface.deref(),
            MorphemeSurface::Headword(surface) => surface,
        }
    }
}

impl AsRef<str> for MorphemeSurface<'_> {
    fn as_ref(&self) -> &str {
        self.deref()
    }
}

impl Debug for MorphemeSurface<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self.deref(), f)
    }
}

impl Display for MorphemeSurface<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self.deref(), f)
    }
}

mod private {
    pub trait Sealed {}
}

/// Common accessor interface for morphemes.
///
/// This trait is implemented by morphemes that are part of an analysis result
/// (`Morpheme`) and by standalone morphemes materialized from a
/// dictionary entry (`SingleMorpheme`).
pub trait MorphemeView: private::Sealed {
    type Dictionary: DictionaryAccess;

    #[doc(hidden)]
    fn dict(&self) -> &Self::Dictionary;

    #[doc(hidden)]
    fn subset(&self) -> InfoSubset;

    /// Returns the begin index in bytes of the morpheme.
    fn begin(&self) -> usize;

    /// Returns the end index in bytes of the morpheme.
    fn end(&self) -> usize;

    /// Returns the codepoint offset of the morpheme begin.
    fn begin_c(&self) -> usize;

    /// Returns the codepoint offset of the morpheme end.
    fn end_c(&self) -> usize;

    /// Returns text corresponding to the morpheme.
    fn surface(&self) -> MorphemeSurface<'_>;

    /// Returns the ID of part of speech of the morpheme.
    fn part_of_speech_id(&self) -> u16 {
        self.get_word_info().pos_id()
    }

    /// Returns the word id of morpheme.
    fn word_id(&self) -> WordId;

    /// Returns the dictionary information for this morpheme.
    fn get_word_info(&self) -> &WordInfo;

    fn self_morpheme(&self) -> MorphemeRef<'_, Self::Dictionary>;

    fn resolver<'a>(&'a self) -> &'a dyn WordInfoResolver
    where
        Self::Dictionary: 'a,
    {
        self.dict().lexicon()
    }

    /// Returns the part of speech.
    fn part_of_speech<'a>(&'a self) -> &'a [String]
    where
        Self::Dictionary: 'a,
    {
        self.dict()
            .grammar()
            .pos_components(self.part_of_speech_id())
    }

    /// Returns the dictionary form of morpheme.
    ///
    /// "Dictionary form" means a word's lemma and "終止形" in Japanese.
    fn dictionary_form<'a>(&'a self) -> &'a str
    where
        Self::Dictionary: 'a,
    {
        self.get_word_info().dictionary_form(self.resolver())
    }

    /// Returns the morpheme corresponding to this morpheme's dictionary form.
    ///
    /// For OOV morphemes, invalid references, and references to the same
    /// dictionary entry, returns a morpheme equivalent to `self`. For a
    /// distinct referenced entry, returns a standalone morpheme whose offsets
    /// are `0..surface.len()` in bytes and `0..surface.chars().count()` in
    /// codepoints.
    #[allow(clippy::result_large_err)]
    fn dictionary_form_morpheme(&self) -> SudachiResult<MorphemeRef<'_, Self::Dictionary>>
    where
        Self::Dictionary: Clone,
    {
        resolve_referenced_form_morpheme(
            self,
            InfoSubset::DICTIONARY_FORM,
            WordInfoData::dictionary_form_word_id,
        )
    }

    /// Returns the normalized form of morpheme.
    ///
    /// This method returns the form normalizing inconsistent spellings and
    /// inflected forms.
    fn normalized_form<'a>(&'a self) -> &'a str
    where
        Self::Dictionary: 'a,
    {
        self.get_word_info().normalized_form(self.resolver())
    }

    /// Returns the morpheme corresponding to this morpheme's normalized form.
    ///
    /// For OOV morphemes, invalid references, and references to the same
    /// dictionary entry, returns a morpheme equivalent to `self`. For a
    /// distinct referenced entry, returns a standalone morpheme whose offsets
    /// are `0..surface.len()` in bytes and `0..surface.chars().count()` in
    /// codepoints.
    #[allow(clippy::result_large_err)]
    fn normalized_form_morpheme(&self) -> SudachiResult<MorphemeRef<'_, Self::Dictionary>>
    where
        Self::Dictionary: Clone,
    {
        resolve_referenced_form_morpheme(
            self,
            InfoSubset::NORMALIZED_FORM,
            WordInfoData::normalized_form_word_id,
        )
    }

    /// Returns the reading form of morpheme.
    ///
    /// Returns Japanese syllabaries 'フリガナ' in katakana.
    fn reading_form<'a>(&'a self) -> &'a str
    where
        Self::Dictionary: 'a,
    {
        self.get_word_info().reading_form(self.resolver())
    }

    /// Returns if this morpheme is out of vocabulary.
    fn is_oov(&self) -> bool {
        self.word_id().is_oov()
    }

    /// Returns the dictionary id where the morpheme belongs.
    ///
    /// Returns -1 if the morpheme is oov.
    fn dictionary_id(&self) -> i32 {
        let wid = self.word_id();
        if wid.is_oov() {
            -1
        } else {
            wid.dict().as_raw() as i32
        }
    }

    fn synonym_group_ids(&self) -> &[i32] {
        self.get_word_info().synonym_group_ids()
    }

    /// Returns user-defined data associated with this morpheme.
    fn user_data(&self) -> &str {
        self.get_word_info().user_data()
    }
}

pub(crate) fn validate_dictionary_word_id<D: DictionaryAccess>(
    dict: &D,
    word_id: WordId,
) -> SudachiResult<()> {
    if word_id == WordId::INVALID || word_id.is_oov() || word_id.is_special() {
        return Err(SudachiError::InvalidWordId(word_id));
    }

    dict.lexicon().get_word_param_checked(word_id).map(|_| ())
}

#[allow(clippy::result_large_err)]
fn resolve_referenced_form_morpheme<'a, M, F>(
    morpheme: &'a M,
    reference_subset: InfoSubset,
    form_word_id: F,
) -> SudachiResult<MorphemeRef<'a, M::Dictionary>>
where
    M: MorphemeView + ?Sized,
    M::Dictionary: Clone,
    F: FnOnce(&WordInfoData) -> WordId,
{
    let word_id = morpheme.word_id();
    if word_id.is_oov() || word_id.is_special() {
        return Ok(morpheme.self_morpheme());
    }

    let word_info = morpheme
        .dict()
        .lexicon()
        .get_word_info_subset(word_id, reference_subset)?;
    let form_word_id = form_word_id(word_info.borrow_data());
    if form_word_id == WordId::INVALID
        || form_word_id == word_id
        || form_word_id.is_oov()
        || form_word_id.is_special()
    {
        return Ok(morpheme.self_morpheme());
    }

    let materialized_subset = (morpheme.subset() | InfoSubset::HEADWORD).normalize();

    SingleMorpheme::from_word_id(morpheme.dict().clone(), form_word_id, materialized_subset)
        .map(Box::new)
        .map(MorphemeRef::Single)
}

/// A morpheme as a part of an analysis result.
pub struct Morpheme<'a, D> {
    list: &'a MorphemeList<D>,
    index: usize,
}

impl<D> Clone for Morpheme<'_, D> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<D> Copy for Morpheme<'_, D> {}

impl<D: DictionaryAccess + Clone> Morpheme<'_, D> {
    /// Returns new morpheme list splitting the morpheme with given mode.
    #[deprecated(note = "use split_into", since = "0.6.1")]
    #[allow(clippy::result_large_err)]
    pub fn split(&self, mode: Mode) -> SudachiResult<MorphemeList<D>> {
        #[allow(deprecated)]
        self.list.split(mode, self.index)
    }
}

impl<'a, D: DictionaryAccess> Morpheme<'a, D> {
    pub(crate) fn for_list(list: &'a MorphemeList<D>, index: usize) -> Self {
        Morpheme { list, index }
    }

    #[inline]
    pub(crate) fn node(&self) -> &ResultNode {
        self.list.node(self.index)
    }

    /// Returns the part of speech.
    pub fn part_of_speech(&self) -> &[String] {
        <Self as MorphemeView>::part_of_speech(self)
    }

    /// Returns the begin index in bytes of the morpheme in the original text.
    pub fn begin(&self) -> usize {
        self.list.input().to_orig_byte_idx(self.node().begin())
    }

    /// Returns the end index in bytes of the morpheme in the original text.
    pub fn end(&self) -> usize {
        self.list.input().to_orig_byte_idx(self.node().end())
    }

    /// Returns the codepoint offset of the morpheme begin in the original text.
    pub fn begin_c(&self) -> usize {
        self.list.input().to_orig_char_idx(self.node().begin())
    }

    /// Returns the codepoint offset of the morpheme end in the original text.
    pub fn end_c(&self) -> usize {
        self.list.input().to_orig_char_idx(self.node().end())
    }

    /// Returns a substring of the original text which corresponds to the morpheme.
    pub fn surface(&self) -> Ref<'_, str> {
        let inp = self.list.input();
        Ref::map(inp, |i| i.orig_slice(self.node().bytes_range()))
    }

    pub fn part_of_speech_id(&self) -> u16 {
        <Self as MorphemeView>::part_of_speech_id(self)
    }

    /// Returns the dictionary form of morpheme.
    ///
    /// "Dictionary form" means a word's lemma and "終止形" in Japanese.
    pub fn dictionary_form(&self) -> &str {
        <Self as MorphemeView>::dictionary_form(self)
    }

    /// Returns the morpheme corresponding to this morpheme's dictionary form.
    #[allow(clippy::result_large_err)]
    pub fn dictionary_form_morpheme(&self) -> SudachiResult<MorphemeRef<'_, D>>
    where
        D: Clone,
    {
        <Self as MorphemeView>::dictionary_form_morpheme(self)
    }

    /// Returns the normalized form of morpheme.
    ///
    /// This method returns the form normalizing inconsistent spellings and
    /// inflected forms.
    pub fn normalized_form(&self) -> &str {
        <Self as MorphemeView>::normalized_form(self)
    }

    /// Returns the morpheme corresponding to this morpheme's normalized form.
    #[allow(clippy::result_large_err)]
    pub fn normalized_form_morpheme(&self) -> SudachiResult<MorphemeRef<'_, D>>
    where
        D: Clone,
    {
        <Self as MorphemeView>::normalized_form_morpheme(self)
    }

    /// Returns the reading form of morpheme.
    ///
    /// Returns Japanese syllabaries 'フリガナ' in katakana.
    pub fn reading_form(&self) -> &str {
        <Self as MorphemeView>::reading_form(self)
    }

    /// Returns if this morpheme is out of vocabulary.
    pub fn is_oov(&self) -> bool {
        <Self as MorphemeView>::is_oov(self)
    }

    /// Returns the word id of morpheme.
    pub fn word_id(&self) -> WordId {
        self.node().word_id()
    }

    /// Returns the dictionary id where the morpheme belongs.
    ///
    /// Returns -1 if the morpheme is oov.
    pub fn dictionary_id(&self) -> i32 {
        <Self as MorphemeView>::dictionary_id(self)
    }

    pub fn synonym_group_ids(&self) -> &[i32] {
        <Self as MorphemeView>::synonym_group_ids(self)
    }

    /// Returns user-defined data associated with this morpheme.
    pub fn user_data(&self) -> &str {
        <Self as MorphemeView>::user_data(self)
    }

    pub fn get_word_info(&self) -> &WordInfo {
        self.node().word_info()
    }

    /// Returns the index of this morpheme.
    pub fn index(&self) -> usize {
        self.index
    }

    /// Splits morpheme and writes sub-morphemes into the provided list.
    /// The resulting list is _not_ cleared before that.
    /// Returns true if split has produced any elements.
    #[allow(clippy::result_large_err)]
    pub fn split_into(&self, mode: Mode, out: &mut MorphemeList<D>) -> SudachiResult<bool> {
        self.list.split_into(mode, self.index, out)
    }

    /// Returns total cost from the beginning of the path.
    pub fn total_cost(&self) -> i32 {
        self.node().total_cost()
    }
}

impl<D> private::Sealed for Morpheme<'_, D> {}

impl<D: DictionaryAccess> MorphemeView for Morpheme<'_, D> {
    type Dictionary = D;

    fn dict(&self) -> &D {
        self.list.dict()
    }

    fn subset(&self) -> InfoSubset {
        self.list.subset()
    }

    fn begin(&self) -> usize {
        Morpheme::begin(self)
    }

    fn end(&self) -> usize {
        Morpheme::end(self)
    }

    fn begin_c(&self) -> usize {
        Morpheme::begin_c(self)
    }

    fn end_c(&self) -> usize {
        Morpheme::end_c(self)
    }

    fn surface(&self) -> MorphemeSurface<'_> {
        MorphemeSurface::Input(Morpheme::surface(self))
    }

    fn word_id(&self) -> WordId {
        Morpheme::word_id(self)
    }

    fn get_word_info(&self) -> &WordInfo {
        Morpheme::get_word_info(self)
    }

    fn self_morpheme(&self) -> MorphemeRef<'_, D> {
        MorphemeRef::ListItem(*self)
    }
}

impl<D: DictionaryAccess> Debug for Morpheme<'_, D> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Morpheme")
            .field("surface", &self.surface())
            .field("pos", &self.part_of_speech())
            .field("normalized_form", &self.normalized_form())
            .field("reading_form", &self.reading_form())
            .field("dictionary_form", &self.dictionary_form())
            .finish()
    }
}

/// A standalone morpheme materialized from a single dictionary entry.
pub struct SingleMorpheme<D> {
    dict: D,
    word_id: WordId,
    word_info: WordInfo,
    subset: InfoSubset,
    begin: usize,
    end: usize,
    begin_c: usize,
    end_c: usize,
}

impl<D: Clone> Clone for SingleMorpheme<D> {
    fn clone(&self) -> Self {
        Self {
            dict: self.dict.clone(),
            word_id: self.word_id,
            word_info: self.word_info.clone(),
            subset: self.subset,
            begin: self.begin,
            end: self.end,
            begin_c: self.begin_c,
            end_c: self.end_c,
        }
    }
}

impl<D: DictionaryAccess> SingleMorpheme<D> {
    /// Creates a standalone morpheme for the exact dictionary entry.
    ///
    /// The entry is resolved by `WordId`, not by surface lookup, so homograph
    /// identity is preserved. `HEADWORD` is always loaded because standalone
    /// offsets and surface are based on the dictionary headword.
    #[allow(clippy::result_large_err)]
    pub fn from_word_id(dict: D, word_id: WordId, subset: InfoSubset) -> SudachiResult<Self> {
        validate_dictionary_word_id(&dict, word_id)?;
        let subset = (subset | InfoSubset::HEADWORD).normalize();
        let word_info = dict.lexicon().get_word_info_subset(word_id, subset)?;
        let surface = word_info.headword(dict.lexicon());
        let end = surface.len();
        let end_c = surface.chars().count();

        Ok(Self {
            dict,
            word_id,
            word_info,
            subset,
            begin: 0,
            end,
            begin_c: 0,
            end_c,
        })
    }

    pub fn oov(
        dict: D,
        pos_id: u16,
        surface: String,
        reading: String,
        normalized_form: String,
        dictionary_form: String,
    ) -> SudachiResult<Self> {
        if pos_id as usize >= dict.grammar().pos_list.len() {
            return Err(SudachiError::InvalidPartOfSpeech(pos_id.to_string()));
        }
        let end = surface.len();
        let end_c = surface.chars().count();
        let word_id = WordId::oov(pos_id as u32);
        let word_info = WordInfo::new_with_strings(
            pos_id as i16,
            end as i16,
            word_id,
            surface,
            reading,
            normalized_form,
            dictionary_form,
        );

        Ok(Self {
            dict,
            word_id,
            word_info,
            subset: InfoSubset::all(),
            begin: 0,
            end,
            begin_c: 0,
            end_c,
        })
    }

    pub(crate) fn subset(&self) -> InfoSubset {
        self.subset
    }

    pub(crate) fn dict(&self) -> &D {
        &self.dict
    }

    /// Returns the begin index in bytes of this standalone morpheme.
    pub fn begin(&self) -> usize {
        self.begin
    }

    /// Returns the end index in bytes of this standalone morpheme.
    pub fn end(&self) -> usize {
        self.end
    }

    /// Returns the codepoint offset of this standalone morpheme begin.
    pub fn begin_c(&self) -> usize {
        self.begin_c
    }

    /// Returns the codepoint offset of this standalone morpheme end.
    pub fn end_c(&self) -> usize {
        self.end_c
    }

    /// Returns the dictionary headword surface.
    pub fn surface(&self) -> &str {
        self.word_info.headword(self.dict.lexicon())
    }

    /// Returns the word id of morpheme.
    pub fn word_id(&self) -> WordId {
        self.word_id
    }

    pub fn get_word_info(&self) -> &WordInfo {
        &self.word_info
    }
}

impl<D: DictionaryAccess + Clone> SingleMorpheme<D> {
    /// Returns the part of speech.
    pub fn part_of_speech(&self) -> &[String] {
        <Self as MorphemeView>::part_of_speech(self)
    }

    /// Returns standalone sub-morphemes for the requested split mode.
    ///
    /// When the dictionary entry has no splits for the mode, returns a clone of
    /// this morpheme. Offsets match Java's standalone morpheme behavior: a
    /// single replacement split preserves this morpheme's span, while
    /// multi-splits advance over each split surface.
    #[allow(clippy::result_large_err)]
    pub fn split(&self, mode: Mode) -> SudachiResult<Vec<SingleMorpheme<D>>> {
        let split_subset = match mode {
            Mode::A => InfoSubset::SPLIT_A,
            Mode::B => InfoSubset::SPLIT_B,
            Mode::C => return Ok(vec![self.clone()]),
        };

        let word_info = if self.subset.contains(split_subset) {
            Cow::Borrowed(&self.word_info)
        } else {
            Cow::Owned(
                self.dict
                    .lexicon()
                    .get_word_info_subset(self.word_id, (self.subset | split_subset).normalize())?,
            )
        };

        let splits = if mode == Mode::A {
            word_info.a_unit_split()
        } else {
            word_info.b_unit_split()
        };

        if splits.is_empty() {
            return Ok(vec![self.clone()]);
        }

        if let [word_id] = splits {
            if *word_id == self.word_id {
                return Ok(vec![self.clone()]);
            }

            let mut morpheme =
                SingleMorpheme::from_word_id(self.dict.clone(), *word_id, self.subset)?;
            morpheme.begin = self.begin;
            morpheme.end = self.end;
            morpheme.begin_c = self.begin_c;
            morpheme.end_c = self.end_c;
            return Ok(vec![morpheme]);
        }

        let mut result = Vec::with_capacity(splits.len());
        let mut begin = self.begin;
        let mut begin_c = self.begin_c;
        for &word_id in splits {
            let mut morpheme =
                SingleMorpheme::from_word_id(self.dict.clone(), word_id, self.subset)?;
            let end = begin + morpheme.surface().len();
            let end_c = begin_c + morpheme.surface().chars().count();
            morpheme.begin = begin;
            morpheme.end = end;
            morpheme.begin_c = begin_c;
            morpheme.end_c = end_c;
            begin = end;
            begin_c = end_c;
            result.push(morpheme);
        }

        Ok(result)
    }

    pub fn part_of_speech_id(&self) -> u16 {
        <Self as MorphemeView>::part_of_speech_id(self)
    }

    /// Returns the dictionary form of morpheme.
    pub fn dictionary_form(&self) -> &str {
        <Self as MorphemeView>::dictionary_form(self)
    }

    /// Returns the morpheme corresponding to this morpheme's dictionary form.
    #[allow(clippy::result_large_err)]
    pub fn dictionary_form_morpheme(&self) -> SudachiResult<MorphemeRef<'_, D>> {
        <Self as MorphemeView>::dictionary_form_morpheme(self)
    }

    /// Returns the normalized form of morpheme.
    pub fn normalized_form(&self) -> &str {
        <Self as MorphemeView>::normalized_form(self)
    }

    /// Returns the morpheme corresponding to this morpheme's normalized form.
    #[allow(clippy::result_large_err)]
    pub fn normalized_form_morpheme(&self) -> SudachiResult<MorphemeRef<'_, D>> {
        <Self as MorphemeView>::normalized_form_morpheme(self)
    }

    /// Returns the reading form of morpheme.
    pub fn reading_form(&self) -> &str {
        <Self as MorphemeView>::reading_form(self)
    }

    /// Returns if this morpheme is out of vocabulary.
    pub fn is_oov(&self) -> bool {
        <Self as MorphemeView>::is_oov(self)
    }

    /// Returns the dictionary id where the morpheme belongs.
    pub fn dictionary_id(&self) -> i32 {
        <Self as MorphemeView>::dictionary_id(self)
    }

    pub fn synonym_group_ids(&self) -> &[i32] {
        <Self as MorphemeView>::synonym_group_ids(self)
    }

    /// Returns user-defined data associated with this morpheme.
    pub fn user_data(&self) -> &str {
        <Self as MorphemeView>::user_data(self)
    }
}

impl<D> private::Sealed for SingleMorpheme<D> {}

impl<D: DictionaryAccess + Clone> MorphemeView for SingleMorpheme<D> {
    type Dictionary = D;

    fn dict(&self) -> &D {
        &self.dict
    }

    fn subset(&self) -> InfoSubset {
        self.subset
    }

    fn begin(&self) -> usize {
        SingleMorpheme::begin(self)
    }

    fn end(&self) -> usize {
        SingleMorpheme::end(self)
    }

    fn begin_c(&self) -> usize {
        SingleMorpheme::begin_c(self)
    }

    fn end_c(&self) -> usize {
        SingleMorpheme::end_c(self)
    }

    fn surface(&self) -> MorphemeSurface<'_> {
        MorphemeSurface::Headword(SingleMorpheme::surface(self))
    }

    fn word_id(&self) -> WordId {
        SingleMorpheme::word_id(self)
    }

    fn get_word_info(&self) -> &WordInfo {
        SingleMorpheme::get_word_info(self)
    }

    fn self_morpheme(&self) -> MorphemeRef<'_, D> {
        MorphemeRef::Single(Box::new(self.clone()))
    }
}

impl<D: DictionaryAccess + Clone> Debug for SingleMorpheme<D> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SingleMorpheme")
            .field("surface", &self.surface())
            .field("pos", &self.part_of_speech())
            .field("normalized_form", &self.normalized_form())
            .field("reading_form", &self.reading_form())
            .field("dictionary_form", &self.dictionary_form())
            .finish()
    }
}

/// A morpheme reference that can be either list-backed or standalone.
#[non_exhaustive]
pub enum MorphemeRef<'a, D> {
    ListItem(Morpheme<'a, D>),
    Single(Box<SingleMorpheme<D>>),
}

impl<D: Clone> Clone for MorphemeRef<'_, D> {
    fn clone(&self) -> Self {
        match self {
            MorphemeRef::ListItem(m) => MorphemeRef::ListItem(*m),
            MorphemeRef::Single(m) => MorphemeRef::Single(m.clone()),
        }
    }
}

impl<D: DictionaryAccess + Clone> MorphemeRef<'_, D> {
    /// Returns user-defined data associated with this morpheme.
    pub fn user_data(&self) -> &str {
        <Self as MorphemeView>::user_data(self)
    }
}

impl<D> private::Sealed for MorphemeRef<'_, D> {}

impl<D: DictionaryAccess + Clone> MorphemeView for MorphemeRef<'_, D> {
    type Dictionary = D;

    fn dict(&self) -> &D {
        match self {
            MorphemeRef::ListItem(m) => m.dict(),
            MorphemeRef::Single(m) => m.dict(),
        }
    }

    fn subset(&self) -> InfoSubset {
        match self {
            MorphemeRef::ListItem(m) => m.subset(),
            MorphemeRef::Single(m) => m.subset(),
        }
    }

    fn begin(&self) -> usize {
        match self {
            MorphemeRef::ListItem(m) => m.begin(),
            MorphemeRef::Single(m) => m.begin(),
        }
    }

    fn end(&self) -> usize {
        match self {
            MorphemeRef::ListItem(m) => m.end(),
            MorphemeRef::Single(m) => m.end(),
        }
    }

    fn begin_c(&self) -> usize {
        match self {
            MorphemeRef::ListItem(m) => m.begin_c(),
            MorphemeRef::Single(m) => m.begin_c(),
        }
    }

    fn end_c(&self) -> usize {
        match self {
            MorphemeRef::ListItem(m) => m.end_c(),
            MorphemeRef::Single(m) => m.end_c(),
        }
    }

    fn surface(&self) -> MorphemeSurface<'_> {
        match self {
            MorphemeRef::ListItem(m) => MorphemeSurface::Input(m.surface()),
            MorphemeRef::Single(m) => MorphemeSurface::Headword(m.surface()),
        }
    }

    fn word_id(&self) -> WordId {
        match self {
            MorphemeRef::ListItem(m) => m.word_id(),
            MorphemeRef::Single(m) => m.word_id(),
        }
    }

    fn get_word_info(&self) -> &WordInfo {
        match self {
            MorphemeRef::ListItem(m) => m.get_word_info(),
            MorphemeRef::Single(m) => m.get_word_info(),
        }
    }

    fn self_morpheme(&self) -> MorphemeRef<'_, D> {
        self.clone()
    }
}

impl<D: DictionaryAccess + Clone> Debug for MorphemeRef<'_, D> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            MorphemeRef::ListItem(m) => Debug::fmt(m, f),
            MorphemeRef::Single(m) => Debug::fmt(m, f),
        }
    }
}
