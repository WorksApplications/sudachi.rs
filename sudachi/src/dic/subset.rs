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

use bitflags::bitflags;

bitflags! {
    #[repr(transparent)]
    pub struct InfoSubset: u32 {
        const SURFACE = (1 << 0);
        const HEAD_WORD_LENGTH = (1 << 1);
        const POS_ID = (1 << 2);
        const NORMALIZED_FORM = (1 << 3);
        const DIC_FORM_WORD_ID = (1 << 4);
        const READING_FORM = (1 << 5);
        const SPLIT_A = (1 << 6);
        const SPLIT_B = (1 << 7);
        const WORD_STRUCTURE = (1 << 8);
        const SYNONYM_GROUP_ID = (1 << 9);
    }
}

impl Default for InfoSubset {
    fn default() -> Self {
        Self::all()
    }
}

impl InfoSubset {
    pub fn normalize(mut self) -> Self {
        // need to read surface if reading any of one of these forms
        if self.intersects(InfoSubset::READING_FORM | InfoSubset::NORMALIZED_FORM) {
            self |= InfoSubset::SURFACE
        }

        // need to have head word length when splitting
        if self.intersects(InfoSubset::SPLIT_A | InfoSubset::SPLIT_B) {
            self |= InfoSubset::HEAD_WORD_LENGTH;
        }

        self
    }
}
