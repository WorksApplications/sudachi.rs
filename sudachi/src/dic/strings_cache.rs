/*
 *  Copyright (c) 2026 Works Applications Co., Ltd.
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

use std::sync::OnceLock;

use crate::dic::subset::InfoSubset;
use crate::dic::word_id::WordId;
use crate::dic::word_info::{WordInfoData, WordInfoResolver};

#[derive(Clone, Debug, Default)]
pub(super) struct StringsCache {
    headword: OnceLock<String>,
    reading: OnceLock<String>,
    normalized_form: OnceLock<String>,
    dictionary_form: OnceLock<String>,
}

impl StringsCache {
    /// Creates a new StringsCache for the WordId
    pub fn new() -> Self {
        StringsCache {
            headword: OnceLock::new(),
            reading: OnceLock::new(),
            normalized_form: OnceLock::new(),
            dictionary_form: OnceLock::new(),
        }
    }

    /// Creates a new StringsCache for the WordId with all strings pre-fetched
    pub fn new_with_strings(
        headword: String,
        reading: String,
        normalized_form: String,
        dictionary_form: String,
    ) -> Self {
        StringsCache {
            headword: {
                let lock = OnceLock::new();
                lock.set(headword).unwrap();
                lock
            },
            reading: {
                let lock = OnceLock::new();
                lock.set(reading).unwrap();
                lock
            },
            normalized_form: {
                let lock = OnceLock::new();
                lock.set(normalized_form).unwrap();
                lock
            },
            dictionary_form: {
                let lock = OnceLock::new();
                lock.set(dictionary_form).unwrap();
                lock
            },
        }
    }

    pub fn new_with_single_string(headword: String) -> Self {
        StringsCache::new_with_strings(
            headword.clone(),
            headword.clone(),
            headword.clone(),
            headword,
        )
    }
}

impl StringsCache {
    pub fn headword(
        &self,
        resolver: &dyn WordInfoResolver,
        word_info: &WordInfoData,
        word_id: WordId,
    ) -> &str {
        self.headword.get_or_init(|| {
            let strptr = word_info.headword_strptr();
            resolver
                .lexicon()
                .get_string(word_id, strptr)
                .expect("Headword must exist for non-OOV word IDs")
        })
    }

    pub fn reading(
        &self,
        resolver: &dyn WordInfoResolver,
        word_info: &WordInfoData,
        word_id: WordId,
    ) -> &str {
        self.reading.get_or_init(|| {
            let strptr = word_info.reading_form_strptr();
            resolver
                .lexicon()
                .get_string(word_id, strptr)
                .expect("Reading form must exist for non-OOV word IDs")
        })
    }

    pub fn normalized_form(
        &self,
        resolver: &dyn WordInfoResolver,
        word_info: &WordInfoData,
        word_id: WordId,
    ) -> &str {
        self.normalized_form.get_or_init(|| {
            let ref_word_id = word_info.normalized_form_word_id();
            let s = if ref_word_id == word_id {
                self.headword(resolver, word_info, word_id).to_string()
            } else {
                let ref_word_info = resolver
                    .lexicon()
                    .get_word_info_subset(ref_word_id, InfoSubset::HEADWORD)
                    .expect("WordInfo must exist for non-OOV word IDs");
                let strptr = ref_word_info.headword_strptr();
                resolver
                    .lexicon()
                    .get_string(ref_word_id, strptr)
                    .expect("Normalized form must exist for non-OOV word IDs")
            };
            s
        })
    }

    pub fn dictionary_form(
        &self,
        resolver: &dyn WordInfoResolver,
        word_info: &WordInfoData,
        word_id: WordId,
    ) -> &str {
        self.dictionary_form.get_or_init(|| {
            let ref_word_id = word_info.dictionary_form_word_id();
            let s = if ref_word_id == word_id {
                self.headword(resolver, word_info, word_id).to_string()
            } else {
                let ref_word_info = resolver
                    .lexicon()
                    .get_word_info_subset(ref_word_id, InfoSubset::HEADWORD)
                    .expect("WordInfo must exist for non-OOV word IDs");
                let strptr = ref_word_info.headword_strptr();
                resolver
                    .lexicon()
                    .get_string(ref_word_id, strptr)
                    .expect("Dictionary form must exist for non-OOV word IDs")
            };
            s
        })
    }
}
