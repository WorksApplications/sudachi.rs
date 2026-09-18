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

use crate::analysis::Mode;
use crate::dic::build::error::{BuildFailure, DicWriteResult};
use crate::dic::word_id::WordRef as DicWordRef;
use crate::dic::word_info::{layout, WordInfoVariableLayout};

use super::refs::{ResolvedWordRef, WordRef};

/// Entry parsed from the lexicon CSV.
///
/// Internal references such as `dic_form` and `splits_*` are kept as
/// unresolved `WordRef` values.
#[derive(Clone)]
pub(crate) struct ParsedLexiconEntry {
    pub left_id: i16,
    pub right_id: i16,
    pub cost: i16,
    pub index_form: String,
    pub headword: Option<String>,
    pub reference_id: Option<String>,
    pub dic_form: WordRef,
    pub norm_form: WordRef,
    pub pos: u16,
    pub splits_a: Vec<WordRef>,
    pub splits_b: Vec<WordRef>,
    #[allow(unused)]
    pub splits_c: Vec<WordRef>,
    pub reading: String,
    #[allow(unused)]
    pub splitting: Mode,
    pub word_structure: Vec<WordRef>,
    pub synonym_groups: Vec<u32>,
    #[allow(unused)]
    pub user_data: String,
}

/// Lexicon entry after internal references have been resolved.
///
/// This stores the `ParsedLexiconEntry` data with references such as
/// `dic_form` and `splits_*` resolved to actual DicWordRef.
pub(crate) struct ResolvedLexiconEntry {
    pub left_id: i16,
    pub right_id: i16,
    pub cost: i16,
    pub index_form: String,
    pub headword: Option<String>,
    pub reference_id: Option<String>,
    pub dic_form: ResolvedWordRef,
    pub norm_form: ResolvedWordRef,
    pub pos: u16,
    pub splits_a: Vec<DicWordRef>,
    pub splits_b: Vec<DicWordRef>,
    #[allow(unused)]
    pub splits_c: Vec<DicWordRef>,
    pub reading: String,
    #[allow(unused)]
    pub splitting: Mode,
    pub word_structure: Vec<DicWordRef>,
    pub synonym_groups: Vec<u32>,
    #[allow(unused)]
    pub user_data: String,
}

impl ParsedLexiconEntry {
    pub fn index_form(&self) -> &str {
        &self.index_form
    }

    pub fn headword(&self) -> &str {
        self.headword
            .as_deref()
            .unwrap_or_else(|| self.index_form())
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn norm_form(&self) -> &str {
        match &self.norm_form {
            WordRef::SelfRef => self.headword(),
            WordRef::Headword(s) => s,
            _ => {
                panic!("normalized_form must be resolved before writing")
            }
        }
    }

    pub fn reading(&self) -> &str {
        &self.reading
    }

    pub fn reference_id(&self) -> Option<&str> {
        self.reference_id.as_deref()
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn should_index(&self) -> bool {
        self.left_id >= 0
    }

    pub fn expected_entry_size(&self) -> usize {
        let variable = variable_layout(
            self.splits_c.len(),
            self.splits_b == self.splits_c,
            self.splits_b.len(),
            self.splits_a == self.splits_b,
            self.splits_a.len(),
            self.word_structure == self.splits_a,
            self.word_structure.len(),
            self.synonym_groups.len(),
            self.user_data.encode_utf16().count(),
        )
        .expect("entry lengths are validated before size computation");

        layout::size_from_variable_layout(variable)
            .expect("entry lengths are validated before size computation")
    }
}

impl ResolvedLexiconEntry {
    pub(super) fn make_phantom(base: &ResolvedLexiconEntry, headword: String) -> Self {
        Self {
            index_form: String::new(),
            headword: Some(headword),
            reference_id: None,
            left_id: -1,
            right_id: -1,
            cost: i16::MAX,
            pos: base.pos,
            reading: base.reading.clone(),
            dic_form: base.dic_form,
            norm_form: ResolvedWordRef::SelfRef,
            splitting: base.splitting,
            splits_a: base.splits_a.clone(),
            splits_b: base.splits_b.clone(),
            splits_c: base.splits_c.clone(),
            word_structure: base.word_structure.clone(),
            synonym_groups: base.synonym_groups.clone(),
            user_data: base.user_data.clone(),
        }
    }

    pub fn is_phantom(&self) -> bool {
        self.index_form.is_empty()
    }

    pub fn index_form(&self) -> &str {
        &self.index_form
    }

    pub fn headword(&self) -> &str {
        self.headword
            .as_deref()
            .unwrap_or_else(|| self.index_form())
    }

    #[allow(dead_code)]
    pub fn norm_form(&self) -> &str {
        match self.norm_form {
            ResolvedWordRef::SelfRef => self.headword(),
            ResolvedWordRef::Ref(_) => {
                panic!("normalized_form must be resolved through the serialized word reference")
            }
        }
    }

    pub fn reading(&self) -> &str {
        &self.reading
    }

    pub fn reference_id(&self) -> Option<&str> {
        self.reference_id.as_deref()
    }

    pub fn should_index(&self) -> bool {
        self.left_id >= 0
    }

    pub fn expected_entry_size(&self) -> usize {
        let variable = self
            .variable_layout()
            .expect("entry lengths are validated before size computation");

        layout::size_from_variable_layout(variable)
            .expect("entry lengths are validated before size computation")
    }

    pub(super) fn variable_layout(&self) -> DicWriteResult<WordInfoVariableLayout> {
        variable_layout(
            self.splits_c.len(),
            self.splits_b == self.splits_c,
            self.splits_b.len(),
            self.splits_a == self.splits_b,
            self.splits_a.len(),
            self.word_structure == self.splits_a,
            self.word_structure.len(),
            self.synonym_groups.len(),
            self.user_data.encode_utf16().count(),
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn variable_layout(
    splits_c_len: usize,
    split_b_eq_c: bool,
    splits_b_len: usize,
    split_a_eq_b: bool,
    splits_a_len: usize,
    word_structure_eq_a: bool,
    word_structure_len: usize,
    synonym_groups_len: usize,
    user_data_units: usize,
) -> DicWriteResult<WordInfoVariableLayout> {
    WordInfoVariableLayout::new(
        splits_c_len,
        split_b_eq_c,
        splits_b_len,
        split_a_eq_b,
        splits_a_len,
        word_structure_eq_a,
        word_structure_len,
        synonym_groups_len,
        user_data_units,
    )
    .ok_or_else(|| {
        if splits_c_len > i8::MAX as usize {
            return BuildFailure::InvalidFieldSize {
                actual: splits_c_len,
                expected: i8::MAX as usize,
                field: "splits_c",
            };
        }
        if splits_b_len > i8::MAX as usize {
            return BuildFailure::InvalidFieldSize {
                actual: splits_b_len,
                expected: i8::MAX as usize,
                field: "splits_b",
            };
        }
        if splits_a_len > i8::MAX as usize {
            return BuildFailure::InvalidFieldSize {
                actual: splits_a_len,
                expected: i8::MAX as usize,
                field: "splits_a",
            };
        }
        if word_structure_len > i8::MAX as usize {
            return BuildFailure::InvalidFieldSize {
                actual: word_structure_len,
                expected: i8::MAX as usize,
                field: "word_structure",
            };
        }
        if synonym_groups_len > i8::MAX as usize {
            return BuildFailure::InvalidFieldSize {
                actual: synonym_groups_len,
                expected: i8::MAX as usize,
                field: "synonym_groups",
            };
        }
        BuildFailure::InvalidFieldSize {
            actual: user_data_units,
            expected: i16::MAX as usize,
            field: "user_data",
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::Mode;
    use crate::dic::lexicon::strings::StringPointer;
    use crate::dic::subset::InfoSubset;
    use crate::dic::word_info::WordInfoParser;

    #[test]
    fn writer_output_is_readable_by_parser() {
        let entry = ResolvedLexiconEntry {
            left_id: 1,
            right_id: 2,
            cost: 3,
            index_form: "東京".to_string(),
            headword: Some("京都".to_string()),
            reference_id: None,
            dic_form: ResolvedWordRef::SelfRef,
            norm_form: ResolvedWordRef::Ref(DicWordRef::new(true, 13)),
            pos: 4,
            splits_a: vec![DicWordRef::new(true, 5), DicWordRef::new(true, 6)],
            splits_b: vec![DicWordRef::new(true, 7)],
            splits_c: vec![],
            reading: "キョウト".to_string(),
            splitting: Mode::B,
            word_structure: vec![DicWordRef::new(true, 8)],
            synonym_groups: vec![10, 11],
            user_data: "meta".to_string(),
        };

        let self_word_id = DicWordRef::new(true, 12);
        let headword = StringPointer::unchecked(2, 0);
        let reading = StringPointer::unchecked(5, 2);

        let mut bytes = vec![0u8; layout::PARAMS_SIZE];
        let written = entry
            .write_rest(&mut bytes, self_word_id, headword, reading)
            .unwrap();

        let parser = WordInfoParser::subset(InfoSubset::all());
        let wi = parser.parse(&bytes).unwrap();
        assert_eq!(wi.pos_id, 4);
        assert_eq!(wi.dictionary_form, self_word_id.as_raw());
        assert_eq!(wi.normalized_form, DicWordRef::new(true, 13).as_raw());
        assert_eq!(
            wi.a_unit_split,
            &[
                DicWordRef::new(true, 5).as_raw(),
                DicWordRef::new(true, 6).as_raw()
            ]
        );
        assert_eq!(wi.b_unit_split, &[DicWordRef::new(true, 7).as_raw()]);
        assert_eq!(wi.word_structure, &[DicWordRef::new(true, 8).as_raw()]);
        assert_eq!(wi.synonym_group_ids, &[10, 11]);
        assert_eq!(wi.user_data, "meta");
        assert!(bytes.len() >= written + layout::PARAMS_SIZE);
    }
}
