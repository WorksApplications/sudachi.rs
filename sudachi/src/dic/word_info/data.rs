/*
 * Copyright (c) 2025-2026 Works Applications Co., Ltd.
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

use crate::dic::lexicon::strings::StringPointer;
use crate::dic::strings_cache::StringsCache;
use crate::dic::subset::InfoSubset;
use crate::dic::word_id::{DictId, WordId, WordRef};
use crate::dic::word_info::WordInfoRawData;
use crate::dic::LexiconAccess;

/// wrapper type that indicates inner data are not resolved for the specific lexicon set.
#[derive(Clone, Debug, Default)]
#[repr(transparent)]
pub struct WordInfoRefData {
    raw: WordInfoRawData,
}

impl WordInfoRefData {
    pub fn from_raw(raw: WordInfoRawData) -> Self {
        WordInfoRefData { raw }
    }

    pub fn headword_strptr(&self) -> StringPointer {
        self.raw.headword_strptr
    }

    /// Convert into WordInfoData resolving part-of-speech and word references.
    pub fn resolve(
        self,
        dict_id: DictId,
        num_system_pos: usize,
        pos_offsets: &[usize],
        subset: InfoSubset,
    ) -> WordInfoData {
        let mut raw = self.raw;

        if subset.contains(InfoSubset::POS_ID) {
            let pos_id = raw.pos_id as usize;
            if dict_id.is_user() && pos_id >= num_system_pos {
                // is user defined part-of-speech
                let pos_id_diff = pos_id - num_system_pos;
                let pos_offset = pos_offsets[dict_id.as_raw() as usize];

                raw.pos_id = (pos_offset + pos_id_diff) as i16;
            }
        }

        if subset.contains(InfoSubset::NORMALIZED_FORM) {
            raw.normalized_form = WordRef::resolve_raw(raw.normalized_form, dict_id);
        }
        if subset.contains(InfoSubset::DICTIONARY_FORM) {
            raw.dictionary_form = WordRef::resolve_raw(raw.dictionary_form, dict_id);
        }

        if subset.contains(InfoSubset::SPLIT_C) {
            Self::resolve_ref_vec(&mut raw.c_unit_split, dict_id);
        }
        if subset.contains(InfoSubset::SPLIT_B) {
            Self::resolve_ref_vec(&mut raw.b_unit_split, dict_id);
        }
        if subset.contains(InfoSubset::SPLIT_A) {
            Self::resolve_ref_vec(&mut raw.a_unit_split, dict_id);
        }
        if subset.contains(InfoSubset::WORD_STRUCTURE) {
            Self::resolve_ref_vec(&mut raw.word_structure, dict_id);
        }

        WordInfoData::from_resolved(raw)
    }

    fn resolve_ref_vec(refs: &mut Vec<u32>, dict_id: DictId) {
        for raw in refs.iter_mut() {
            *raw = WordRef::resolve_raw(*raw, dict_id);
        }
    }
}

/// wrapper type that indicates inner data are resolved for the specific lexicon set.
#[derive(Clone, Debug, Default)]
#[repr(transparent)]
pub struct WordInfoData {
    raw: WordInfoRawData,
}

impl WordInfoData {
    /// WordRefs in the given WortInfoRawData must be resolved.
    pub fn from_resolved(raw: WordInfoRawData) -> Self {
        WordInfoData { raw }
    }

    pub fn new_oov(pos_id: i16, index_form_length: i16) -> Self {
        Self {
            raw: WordInfoRawData {
                pos_id,
                index_form_length,
                ..Default::default()
            },
        }
    }

    pub fn pos_id(&self) -> u16 {
        self.raw.pos_id as u16
    }

    pub fn index_form_length(&self) -> usize {
        self.raw.index_form_length as usize
    }

    pub fn headword_strptr(&self) -> StringPointer {
        self.raw.headword_strptr
    }

    pub fn reading_form_strptr(&self) -> StringPointer {
        self.raw.reading_form_strptr
    }

    pub fn normalized_form_word_id(&self) -> WordId {
        WordId::from_raw(self.raw.normalized_form)
    }

    pub fn dictionary_form_word_id(&self) -> WordId {
        WordId::from_raw(self.raw.dictionary_form)
    }

    pub fn c_unit_split(&self) -> &[WordId] {
        Self::as_word_id_slice(&self.raw.c_unit_split)
    }

    pub fn b_unit_split(&self) -> &[WordId] {
        Self::as_word_id_slice(&self.raw.b_unit_split)
    }

    pub fn a_unit_split(&self) -> &[WordId] {
        Self::as_word_id_slice(&self.raw.a_unit_split)
    }

    pub fn word_structure(&self) -> &[WordId] {
        Self::as_word_id_slice(&self.raw.word_structure)
    }

    fn as_word_id_slice(raw_slice: &[u32]) -> &[WordId] {
        if raw_slice.is_empty() {
            &[]
        } else {
            // values in the slice are resolved and safely casted to WordId
            unsafe {
                std::slice::from_raw_parts(raw_slice.as_ptr() as *const WordId, raw_slice.len())
            }
        }
    }

    pub fn synonym_group_ids(&self) -> &[i32] {
        &self.raw.synonym_group_ids
    }

    pub fn user_data(&self) -> &str {
        &self.raw.user_data
    }
}

/// Data structure needed to resolve references in the WordInfoData.
/// Currently only lexicon set is needed, but it can be extended in the future if needed.
pub trait WordInfoResolver: LexiconAccess {}

impl<T: LexiconAccess> WordInfoResolver for T {}

/// WordInfo API.
///
/// Internal data is not accessible by default, but can be extracted as
/// `let data: WordInfoData = info.into()`.
/// Note: this will consume WordInfo.
#[derive(Clone)]
pub struct WordInfo {
    data: WordInfoData,

    // keep self word_id for the purpose of simplisity
    word_id: WordId,

    // In dict v1, word info contains only string pointers, and the actual strings are resolved via lexicon set.
    // keep them in the cache to avoid redundant lookups.
    strings: StringsCache,
}

impl WordInfo {
    pub fn new(data: WordInfoData, word_id: WordId) -> Self {
        WordInfo {
            data,
            word_id,
            strings: StringsCache::new(),
        }
    }

    pub fn new_oov(pos_id: u16, index_form_length: i16, word_id: WordId, headword: String) -> Self {
        Self {
            data: WordInfoData::new_oov(pos_id as i16, index_form_length),
            word_id,
            strings: StringsCache::new_with_single_string(headword),
        }
    }

    pub fn new_with_strings(
        pos_id: i16,
        index_form_length: i16,
        word_id: WordId,
        headword: String,
        reading: String,
        normalized_form: String,
        dictionary_form: String,
    ) -> Self {
        WordInfo {
            data: WordInfoData::new_oov(pos_id, index_form_length),
            word_id,
            strings: StringsCache::new_with_strings(
                headword,
                reading,
                normalized_form,
                dictionary_form,
            ),
        }
    }

    pub fn headword_strptr(&self) -> StringPointer {
        // provide access to this for the normalized/dictionary form resolution via WorfRef.
        self.data.headword_strptr()
    }

    pub fn index_form_length(&self) -> usize {
        self.data.index_form_length()
    }

    pub fn pos_id(&self) -> u16 {
        self.data.pos_id()
    }

    pub fn headword<T: WordInfoResolver>(&self, resolver: T) -> &str {
        self.strings.headword(&resolver, &self.data, self.word_id)
    }

    pub fn reading_form<T: WordInfoResolver>(&self, resolver: T) -> &str {
        self.strings.reading(&resolver, &self.data, self.word_id)
    }

    pub fn normalized_form<T: WordInfoResolver>(&self, resolver: T) -> &str {
        self.strings
            .normalized_form(&resolver, &self.data, self.word_id)
    }

    pub fn dictionary_form<T: WordInfoResolver>(&self, resolver: T) -> &str {
        self.strings
            .dictionary_form(&resolver, &self.data, self.word_id)
    }

    pub fn a_unit_split(&self) -> &[WordId] {
        self.data.a_unit_split()
    }

    pub fn b_unit_split(&self) -> &[WordId] {
        self.data.b_unit_split()
    }

    pub fn c_unit_split(&self) -> &[WordId] {
        self.data.c_unit_split()
    }

    pub fn word_structure(&self) -> &[WordId] {
        self.data.word_structure()
    }

    pub fn synonym_group_ids(&self) -> &[i32] {
        self.data.synonym_group_ids()
    }

    pub fn user_data(&self) -> &str {
        self.data.user_data()
    }

    pub fn borrow_data(&self) -> &WordInfoData {
        &self.data
    }
}

impl From<WordInfo> for WordInfoData {
    fn from(info: WordInfo) -> Self {
        info.data
    }
}
