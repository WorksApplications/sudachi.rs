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

use crate::dic::word_id::WordRef as DicWordRef;

#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum WordRef {
    // explicit self-reference used for dictionary_form/normalized_form omission.
    SelfRef,
    LineRef(DicWordRef),
    Headword(String),
    EntryKey {
        headword: String,
        pos: u16,
        reading: String,
        reference_id: Option<String>,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) enum ResolvedWordRef {
    Ref(DicWordRef),
    SelfRef,
}

impl WordRef {
    pub(crate) fn matches(
        &self,
        headword: &str,
        pos: u16,
        reading: &str,
        reference_id: Option<&str>,
    ) -> bool {
        match self {
            WordRef::EntryKey {
                headword: h,
                pos: p,
                reading: r,
                reference_id: rid,
            } => h == headword && *p == pos && r == reading && rid.as_deref() == reference_id,
            _ => false,
        }
    }
}

pub(crate) trait WordRefResolver {
    fn resolve(&self, unit: &WordRef) -> Option<DicWordRef> {
        match unit {
            WordRef::SelfRef => None,
            WordRef::LineRef(line_ref) => self.resolve_by_line_ref(*line_ref),
            WordRef::Headword(headword) => self.resolve_by_headword(headword),
            WordRef::EntryKey {
                headword,
                pos,
                reading,
                reference_id,
            } => self.resolve_entry_key(headword, *pos, reading, reference_id.as_deref()),
        }
    }

    fn resolve_by_line_ref(&self, line_ref: DicWordRef) -> Option<DicWordRef>;

    fn resolve_by_headword(&self, headword: &str) -> Option<DicWordRef>;

    fn resolve_entry_key(
        &self,
        headword: &str,
        pos: u16,
        reading: &str,
        reference_id: Option<&str>,
    ) -> Option<DicWordRef>;
}
