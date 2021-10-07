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

use std::sync::Arc;

use pyo3::exceptions::{self, PyException};
use pyo3::prelude::*;
use pyo3::types::PyType;

use sudachi::analysis::morpheme_list::MorphemeList;
use sudachi::dic::dictionary::JapaneseDictionary;

use crate::dictionary::PyDictionary;
use crate::morpheme::PyWordInfo;
use crate::tokenizer::PySplitMode;

type PyMorphemeList = MorphemeList<Arc<JapaneseDictionary>>;

/// A list of morphemes
#[pyclass(module = "sudachi.morpheme", name = "MorphemeList")]
pub struct PyMorphemeListWrapper {
    inner: Arc<PyMorphemeList>,
}

#[pymethods]
impl PyMorphemeListWrapper {
    /// Returns an empty morpheme list with dictionary
    #[classmethod]
    #[pyo3(text_signature = "(&self, dict)")]
    fn empty(_cls: &PyType, dict: PyDictionary) -> Self {
        Self {
            inner: Arc::new(PyMorphemeList::empty(dict.dictionary.clone())),
        }
    }

    /// Returns the total cost of the path
    #[pyo3(text_signature = "(&self)")]
    fn get_total_cost(&self) -> i32 {
        self.inner.get_internal_cost()
    }

    #[pyo3(text_signature = "(&self)")]
    fn size(&self) -> usize {
        self.inner.len()
    }
}

impl From<MorphemeList<Arc<JapaneseDictionary>>> for PyMorphemeListWrapper {
    fn from(morpheme_list: MorphemeList<Arc<JapaneseDictionary>>) -> Self {
        Self {
            inner: Arc::new(morpheme_list),
        }
    }
}

#[pyproto]
impl pyo3::basic::PyObjectProtocol for PyMorphemeListWrapper {
    fn __str__(&self) -> &str {
        // input_text and path may not matches after MorphemeList.split
        let begin = self.inner.get_begin(0);
        let end = self.inner.get_end(self.size() - 1);
        &self.inner.input_text[begin..end]
    }
}

#[pyproto]
impl pyo3::sequence::PySequenceProtocol for PyMorphemeListWrapper {
    fn __len__(&self) -> usize {
        self.size()
    }

    fn __getitem__(&self, idx: isize) -> PyResult<PyMorpheme> {
        let len = self.__len__() as isize;
        if idx < -len || len <= idx {
            return Err(PyErr::new::<exceptions::PyIndexError, _>(format!(
                "index out of range: the len is {} but the index is {}",
                self.__len__(),
                idx
            )));
        }
        let index = if idx < 0 { idx + len } else { idx } as usize;

        Ok(PyMorpheme {
            list: self.inner.clone(),
            index,
        })
    }
}

#[pyproto]
impl pyo3::iter::PyIterProtocol for PyMorphemeListWrapper {
    fn __iter__(slf: PyRef<Self>) -> PyResult<Py<PyMorphemeIter>> {
        Py::new(
            slf.py(),
            PyMorphemeIter {
                list: slf.inner.clone(),
                index: 0,
            },
        )
    }
}

#[pyclass(module = "sudachi.morpheme", name = "MorphemeIter")]
pub struct PyMorphemeIter {
    list: Arc<PyMorphemeList>,
    index: usize,
}

#[pyproto]
impl pyo3::iter::PyIterProtocol for PyMorphemeIter {
    fn __iter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<Self>) -> Option<PyMorpheme> {
        if let None = slf.list.path.get(slf.index) {
            return None;
        }

        let morpheme = PyMorpheme {
            list: slf.list.clone(),
            index: slf.index,
        };

        slf.index += 1;
        Some(morpheme)
    }
}

#[pyclass(module = "sudachi.morpheme", name = "Morpheme")]
pub struct PyMorpheme {
    list: Arc<PyMorphemeList>,
    index: usize,
}

#[pyproto]
impl pyo3::basic::PyObjectProtocol for PyMorpheme {
    fn __str__(&self) -> &str {
        self.surface()
    }
}

#[pymethods]
impl PyMorpheme {
    /// Returns the begin index of this in the input text
    #[pyo3(text_signature = "($self)")]
    fn begin(&self) -> usize {
        self.list.get_begin(self.index)
    }

    /// Returns the end index of this in the input text
    #[pyo3(text_signature = "($self)")]
    fn end(&self) -> usize {
        self.list.get_end(self.index)
    }

    /// Returns the surface
    #[pyo3(text_signature = "(&self)")]
    fn surface(&self) -> &str {
        self.list.get_surface(self.index)
    }

    /// Returns the part of speech
    #[pyo3(text_signature = "($self)")]
    fn part_of_speech(&self) -> Vec<String> {
        self.list
            .dict
            .grammar()
            .pos_list
            .get(self.part_of_speech_id() as usize)
            .unwrap()
            .clone()
    }

    /// Returns the id of the part of speech in the dictionary
    #[pyo3(text_signature = "($self)")]
    fn part_of_speech_id(&self) -> u16 {
        self.list.get_word_info(self.index).pos_id
    }

    /// Returns the dictionary form
    #[pyo3(text_signature = "($self)")]
    fn dictionary_form(&self) -> &str {
        &self.list.get_word_info(self.index).dictionary_form
    }

    /// Returns the normalized form
    #[pyo3(text_signature = "($self)")]
    fn normalized_form(&self) -> &str {
        &self.list.get_word_info(self.index).normalized_form
    }

    /// Returns the reading form
    #[pyo3(text_signature = "($self)")]
    fn reading_form(&self) -> &str {
        &self.list.get_word_info(self.index).reading_form
    }

    /// Returns a list of morphemes splitting itself with given split mode
    #[pyo3(text_signature = "($self, mode, /)")]
    fn split(&self, mode: PySplitMode) -> PyResult<PyMorphemeListWrapper> {
        Ok(self
            .list
            .split(mode.into(), self.index)
            .map_err(|e| {
                PyException::new_err(format!("Error while splitting morpheme: {}", e.to_string()))
            })?
            .into())
    }

    /// Returns whether if this is out of vocabulary word
    #[pyo3(text_signature = "($self)")]
    fn is_oov(&self) -> bool {
        self.list.is_oov(self.index)
    }

    /// Returns word id of this word in the dictionary
    #[pyo3(text_signature = "($self)")]
    fn word_id(&self) -> u32 {
        self.list.get_node(self.index).word_id.unwrap()
    }

    /// Returns the dictionary id which this word belongs
    #[pyo3(text_signature = "($self)")]
    fn dictionary_id(&self) -> i32 {
        self.list.get_node(self.index).get_dictionary_id()
    }

    /// Returns the list of synonym group ids
    #[pyo3(text_signature = "($self)")]
    fn synonym_group_ids(&self) -> Vec<u32> {
        self.list
            .get_word_info(self.index)
            .synonym_group_ids
            .clone()
    }

    /// Returns the word info
    #[pyo3(text_signature = "($self)")]
    fn get_word_info(&self) -> PyWordInfo {
        self.list.get_word_info(self.index).clone().into()
    }
}
