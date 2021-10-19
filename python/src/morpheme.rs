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
use pyo3::types::{PyList, PyType};

use sudachi::analysis::morpheme::MorphemeList;
use sudachi::analysis::node::LatticeNode;
use sudachi::dic::dictionary::JapaneseDictionary;

use crate::dictionary::PyDictionary;
use crate::tokenizer::PySplitMode;
use crate::word_info::PyWordInfo;

type PyMorphemeList = MorphemeList<Arc<JapaneseDictionary>>;

/// A list of morphemes
#[pyclass(module = "sudachi.morpheme", name = "MorphemeList")]
#[repr(transparent)]
pub struct PyMorphemeListWrapper {
    inner: Arc<PyMorphemeList>,
}

#[pymethods]
impl PyMorphemeListWrapper {
    /// Returns an empty morpheme list with dictionary
    #[classmethod]
    #[pyo3(text_signature = "(dict) -> sudachi.MorphemeList")]
    fn empty(_cls: &PyType, py: Python, dict: &PyDictionary) -> PyResult<Self> {
        let cat = PyModule::import(py, "builtins")?.getattr("DeprecationWarning")?;
        PyErr::warn(py, cat, "Users should not generate MorphemeList by themselves. Use Tokenizer.tokenize(\"\") if you need.", 1)?;

        Ok(Self {
            inner: Arc::new(PyMorphemeList::empty(
                dict.dictionary.as_ref().unwrap().clone(),
            )),
        })
    }

    /// Returns the total cost of the path
    #[pyo3(text_signature = "($self)")]
    fn get_internal_cost(&self) -> i32 {
        self.inner.get_internal_cost()
    }

    /// Returns the number of morpheme in this list.
    #[pyo3(text_signature = "($self)")]
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
        self.inner.surface()
    }
}

#[pyproto]
impl pyo3::sequence::PySequenceProtocol for PyMorphemeListWrapper {
    fn __len__(&self) -> usize {
        self.size()
    }

    fn __getitem__(&self, idx: isize) -> PyResult<PyMorpheme> {
        // pyo3 automatically adds len when a negative idx is given
        let len = self.__len__() as isize;
        if idx < 0 || len <= idx {
            return Err(PyErr::new::<exceptions::PyIndexError, _>(format!(
                "morphemelist index out of range: the len is {} but the index is {}",
                self.__len__(),
                idx
            )));
        }

        Ok(PyMorpheme {
            list: self.inner.clone(),
            index: idx as usize,
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

/// A morpheme (basic semantic unit of language).
#[pyclass(module = "sudachi.morphemelist", name = "MorphemeIter")]
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
        if slf.index >= slf.list.len() {
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
        // call codepoint version
        self.list.get_node(self.index).begin()
    }

    /// Returns the end index of this in the input text
    #[pyo3(text_signature = "($self)")]
    fn end(&self) -> usize {
        // call codepoint version
        self.list.get_node(self.index).end()
    }

    /// Returns the surface
    #[pyo3(text_signature = "($self)")]
    fn surface(&self) -> &str {
        self.list.get_surface(self.index)
    }

    /// Returns the part of speech
    #[pyo3(text_signature = "($self)")]
    fn part_of_speech(&self, py: Python) -> PyResult<Py<PyList>> {
        let pos_id = self.part_of_speech_id();
        let pos = self
            .list
            .get_grammar()
            .pos_list
            .get(pos_id as usize)
            .ok_or(PyException::new_err(format!("Error pos not found")))?;
        Ok(PyList::new(py, pos).into())
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
    #[pyo3(text_signature = "($self, mode, /) -> sudachi.MorphemeList")]
    fn split(&self, py: Python, mode: PySplitMode) -> PyResult<PyMorphemeListWrapper> {
        let cat = PyModule::import(py, "builtins")?.getattr("DeprecationWarning")?;
        PyErr::warn(
            py,
            cat,
            "API around this functionality will change. See github issue #92 for more.",
            1,
        )?;

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
        self.list.get_node(self.index).word_id().as_raw()
    }

    /// Returns the dictionary id which this word belongs
    #[pyo3(text_signature = "($self)")]
    fn dictionary_id(&self) -> i32 {
        let word_id = self.list.get_node(self.index).word_id();
        if word_id.is_oov() {
            -1
        } else {
            word_id.dic() as i32
        }
    }

    /// Returns the list of synonym group ids
    #[pyo3(text_signature = "($self)")]
    fn synonym_group_ids(&self, py: Python) -> Py<PyList> {
        let ids = &self.list.get_word_info(self.index).synonym_group_ids;
        PyList::new(py, ids).into()
    }

    /// Returns the word info
    #[pyo3(text_signature = "($self) -> sudachi.WordInfo")]
    fn get_word_info(&self, py: Python) -> PyResult<PyWordInfo> {
        let cat = PyModule::import(py, "builtins")?.getattr("DeprecationWarning")?;
        PyErr::warn(py, cat, "Users should not touch the raw WordInfo.", 1)?;

        Ok(self.list.get_word_info(self.index).clone().into())
    }
}
