/*
 *  Copyright (c) 2021-2025 Works Applications Co., Ltd.
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

use bitflags::bitflags;

bitflags! {
    #[repr(transparent)]
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub struct InfoSubset: u32 {
        const POS_ID = (1 << 0);
        const HEADWORD = (1 << 1);
        const READING_FORM = (1 << 2);
        const NORMALIZED_FORM = (1 << 3);
        const DICTIONARY_FORM = (1 << 4);
        const INDEX_FORM_LENGTH = (1 << 5);
        const SPLIT_C = (1 << 6);
        const SPLIT_B = (1 << 7);
        const SPLIT_A = (1 << 8);
        const WORD_STRUCTURE = (1 << 9);
        const SYNONYM_GROUP_IDS = (1 << 10);
        const USER_DATA = (1 << 11);
    }
}

impl Default for InfoSubset {
    fn default() -> Self {
        Self::all()
    }
}

impl InfoSubset {
    pub fn normalize(mut self) -> Self {
        // Normalized and dictionary forms are interpreted relative to the
        // headword string, so parsers need HEADWORD even if callers did not
        // request it explicitly.
        if self.intersects(InfoSubset::NORMALIZED_FORM | InfoSubset::DICTIONARY_FORM) {
            self |= InfoSubset::HEADWORD;
        }

        // Split requests need the index form length because split consumers use
        // it when materializing the higher-level WordInfo view.
        if self.intersects(InfoSubset::SPLIT_A | InfoSubset::SPLIT_B | InfoSubset::SPLIT_C) {
            self |= InfoSubset::INDEX_FORM_LENGTH;
        }

        self
    }
}
