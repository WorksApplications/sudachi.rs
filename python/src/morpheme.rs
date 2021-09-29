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

use std::sync::Arc;

use pyo3::exceptions::PyException;
use pyo3::prelude::*;

use sudachi::dic::dictionary::JapaneseDictionary;
use sudachi::dic::lexicon::word_infos::WordInfo;
use sudachi::dic::lexicon_set::LexiconSet;
use sudachi::prelude::*;

use crate::tokenizer::PySplitMode;

#[pyclass(module = "sudachi.morpheme", name = "Morpheme")]
#[derive(Clone)]
pub struct PyMorpheme {
    // begin: usize,  // Need a reconstruction of Morpheme
    // end: usize,  // Need a reconstruction of Morpheme
    surface: String,
    word_info: WordInfo,
    is_oov: bool,
    // word_id: u32,  // Need a reconstruction of Morpheme
    dictionary_id: i32,
    dict: Arc<JapaneseDictionary>,
}

impl PyMorpheme {
    pub fn new(m: Morpheme, dict: Arc<JapaneseDictionary>) -> Self {
        Self {
            surface: m.surface().clone(),
            word_info: m.word_info,
            is_oov: m.is_oov,
            dictionary_id: m.dictionary_id,
            dict,
        }
    }
}

#[pyproto]
impl pyo3::basic::PyObjectProtocol for PyMorpheme {
    fn __str__(&self) -> PyResult<String> {
        Ok(self.surface.clone())
    }
}

#[pymethods]
impl PyMorpheme {
    // /// Returns the begin index of this in the input text
    // #[pyo3(text_signature = "($self)")]
    // fn begin(&self) -> usize {
    //     self.begin
    // }

    // /// Returns the end index of this in the input text
    // #[pyo3(text_signature = "($self)")]
    // fn end(&self) -> usize {
    //     self.end
    // }

    /// Returns the surface
    #[pyo3(text_signature = "($self)")]
    fn surface(&self) -> &str {
        &self.surface
    }

    /// Returns the part of speech
    #[pyo3(text_signature = "($self)")]
    fn part_of_speech(&self) -> Vec<String> {
        self.dict
            .grammar()
            .pos_list
            .get(self.part_of_speech_id() as usize)
            .unwrap()
            .clone()
    }

    /// Returns the id of the part of speech in the dictionary
    #[pyo3(text_signature = "($self)")]
    fn part_of_speech_id(&self) -> u16 {
        self.word_info.pos_id
    }

    /// Returns the dictionary form
    #[pyo3(text_signature = "($self)")]
    fn dictionary_form(&self) -> &str {
        &self.word_info.dictionary_form
    }

    /// Returns the normalized form
    #[pyo3(text_signature = "($self)")]
    fn normalized_form(&self) -> &str {
        &self.word_info.normalized_form
    }

    /// Returns the reading form
    #[pyo3(text_signature = "($self)")]
    fn reading_form(&self) -> &str {
        &self.word_info.reading_form
    }

    /// Returns a list of morphemes splitting itself with given split mode
    #[pyo3(text_signature = "($self, mode, /)")]
    fn split(&self, mode: PySplitMode) -> PyResult<Vec<PyMorpheme>> {
        let word_ids = match mode {
            PySplitMode::A => &self.word_info.a_unit_split,
            PySplitMode::B => &self.word_info.b_unit_split,
            PySplitMode::C => return Ok(vec![self.clone()]),
            _ => return Err(PyException::new_err(format!("Error invalid SplitMode",))),
        };

        if word_ids.len() < 2 {
            return Ok(vec![self.clone()]);
        }

        let mut morphemes = Vec::with_capacity(word_ids.len());
        for &wid in word_ids {
            let word_info = self.dict.lexicon().get_word_info(wid).map_err(|e| {
                PyException::new_err(format!("Error while getting word_info: {}", e.to_string()))
            })?;

            morphemes.push(PyMorpheme {
                surface: word_info.surface.clone(),
                word_info,
                is_oov: false,
                dictionary_id: LexiconSet::get_dictionary_id(wid) as i32,
                dict: self.dict.clone(),
            });
        }

        Ok(morphemes)
    }

    /// Returns whether if this is out of vocabulary word
    #[pyo3(text_signature = "($self)")]
    fn is_oov(&self) -> bool {
        self.is_oov
    }

    // /// Returns word id of this word in the dictionary
    // #[pyo3(text_signature = "($self)")]
    // fn word_id(&self) -> u32 {
    //     self.word_id
    // }

    /// Returns the dictionary id which this word belongs
    #[pyo3(text_signature = "($self)")]
    fn dictionary_id(&self) -> i32 {
        self.dictionary_id
    }

    /// Returns the list of synonym group ids
    #[pyo3(text_signature = "($self)")]
    fn synonym_group_ids(&self) -> Vec<u32> {
        self.word_info.synonym_group_ids.clone()
    }

    /// Returns the word info
    #[pyo3(text_signature = "($self)")]
    fn get_word_info(&self) -> PyWordInfo {
        self.word_info.clone().into()
    }
}

#[pyclass(module = "sudachi.wordinfo", name = "WordInfo")]
pub struct PyWordInfo {
    #[pyo3(get)]
    surface: String,
    #[pyo3(get)]
    head_word_length: u16,
    #[pyo3(get)]
    pos_id: u16,
    #[pyo3(get)]
    normalized_form: String,
    #[pyo3(get)]
    dictionary_form_word_id: i32,
    #[pyo3(get)]
    dictionary_form: String,
    #[pyo3(get)]
    reading_form: String,
    #[pyo3(get)]
    a_unit_split: Vec<u32>,
    #[pyo3(get)]
    b_unit_split: Vec<u32>,
    #[pyo3(get)]
    word_structure: Vec<u32>,
    #[pyo3(get)]
    synonym_group_ids: Vec<u32>,
}

impl From<WordInfo> for PyWordInfo {
    fn from(word_info: WordInfo) -> Self {
        Self {
            surface: word_info.surface,
            head_word_length: word_info.head_word_length,
            pos_id: word_info.pos_id,
            normalized_form: word_info.normalized_form,
            dictionary_form_word_id: word_info.dictionary_form_word_id,
            dictionary_form: word_info.dictionary_form,
            reading_form: word_info.reading_form,
            a_unit_split: word_info.a_unit_split,
            b_unit_split: word_info.b_unit_split,
            word_structure: word_info.word_structure,
            synonym_group_ids: word_info.synonym_group_ids,
        }
    }
}

#[pymethods]
impl PyWordInfo {
    fn length(&self) -> u16 {
        self.head_word_length
    }
}
