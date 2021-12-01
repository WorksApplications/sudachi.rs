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

use std::ops::Deref;
use std::sync::Arc;

use pyo3::exceptions::{self, PyException};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyString, PyTuple, PyType};

use sudachi::prelude::{Morpheme, MorphemeList};

use crate::dictionary::{PyDicData, PyDictionary};
use crate::tokenizer::PySplitMode;
use crate::word_info::PyWordInfo;

type PyMorphemeList = MorphemeList<Arc<PyDicData>>;

/// A list of morphemes
#[pyclass(module = "sudachipy.morphemelist", name = "MorphemeList")]
#[repr(transparent)]
pub struct PyMorphemeListWrapper {
    inner: PyMorphemeList,
}

// PyMorphemeListWrapper is used only when GIL is active,
// all associated functions take GIL token as a parameter
unsafe impl Sync for PyMorphemeListWrapper {}
unsafe impl Send for PyMorphemeListWrapper {}

impl PyMorphemeListWrapper {
    pub(crate) fn new(dict: Arc<PyDicData>) -> Self {
        Self {
            inner: PyMorphemeList::empty(dict),
        }
    }

    /// Borrow internals mutable. GIL token proves access.
    pub(crate) fn internal_mut(&mut self, _py: Python) -> &mut PyMorphemeList {
        &mut self.inner
    }

    /// Borrow internals immutable. GIL token proves access.
    pub(crate) fn internal(&self, _py: Python) -> &PyMorphemeList {
        &self.inner
    }

    /// Create a copy with empty list of Nodes. GIL token proves access.
    pub(crate) fn empty_clone(&self, _py: Python) -> Self {
        Self {
            inner: self.inner.empty_clone(),
        }
    }
}
#[pymethods]
impl PyMorphemeListWrapper {
    /// Returns an empty morpheme list with dictionary
    #[classmethod]
    #[pyo3(text_signature = "(dict: sudachipy.Dictionary) -> sudachipy.MorphemeList")]
    fn empty(_cls: &PyType, py: Python, dict: &PyDictionary) -> PyResult<Self> {
        let cat = PyModule::import(py, "builtins")?.getattr("DeprecationWarning")?;
        PyErr::warn(py, cat, "Users should not generate MorphemeList by themselves. Use Tokenizer.tokenize(\"\") if you need.", 1)?;

        Ok(Self {
            inner: PyMorphemeList::empty(dict.dictionary.as_ref().unwrap().clone()),
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

    fn __len__(&self) -> usize {
        self.size()
    }

    fn __getitem__(slf: &PyCell<PyMorphemeListWrapper>, mut idx: isize) -> PyResult<PyMorpheme> {
        let list = slf.borrow();
        let len = list.__len__() as isize;

        if idx < 0 {
            // negative indexing
            idx += len;
        }

        if idx < 0 || len <= idx {
            return Err(PyErr::new::<exceptions::PyIndexError, _>(format!(
                "morphemelist index out of range: the len is {} but the index is {}",
                list.__len__(),
                idx
            )));
        }

        let py_list: Py<PyMorphemeListWrapper> = slf.into();

        Ok(PyMorpheme {
            list: py_list,
            index: idx as usize,
        })
    }

    fn __str__<'py>(&'py self, py: Python<'py>) -> &PyString {
        PyString::new(py, self.inner.surface().deref())
    }

    fn __iter__(slf: Py<Self>) -> PyMorphemeIter {
        PyMorphemeIter {
            list: slf,
            index: 0,
        }
    }

    fn __bool__(&self) -> bool {
        self.inner.len() != 0
    }
}

impl From<MorphemeList<Arc<PyDicData>>> for PyMorphemeListWrapper {
    fn from(morpheme_list: MorphemeList<Arc<PyDicData>>) -> Self {
        Self {
            inner: morpheme_list,
        }
    }
}

/// A morpheme (basic semantic unit of language).
#[pyclass(module = "sudachipy.morphemelist", name = "MorphemeIter")]
pub struct PyMorphemeIter {
    list: Py<PyMorphemeListWrapper>,
    index: usize,
}

#[pymethods]
impl PyMorphemeIter {
    fn __iter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    fn __next__(&mut self, py: Python) -> Option<PyMorpheme> {
        if self.index >= self.list.borrow(py).size() {
            return None;
        }

        let morpheme = PyMorpheme {
            list: self.list.clone(),
            index: self.index,
        };

        self.index += 1;
        Some(morpheme)
    }
}

/// It is a syntax sugar for accessing Morpheme reference
/// Without it binding implementations become much less readable
struct MorphemeRef<'py> {
    #[allow(unused)] // need to keep this around for correct reference count
    list: PyRef<'py, PyMorphemeListWrapper>,
    morph: Morpheme<'py, Arc<PyDicData>>,
}

impl<'py> Deref for MorphemeRef<'py> {
    type Target = Morpheme<'py, Arc<PyDicData>>;

    fn deref(&self) -> &Self::Target {
        &self.morph
    }
}

#[pyclass(module = "sudachipy.morpheme", name = "Morpheme")]
pub struct PyMorpheme {
    list: Py<PyMorphemeListWrapper>,
    index: usize,
}

impl PyMorpheme {
    fn list<'py>(&'py self, py: Python<'py>) -> PyRef<'py, PyMorphemeListWrapper> {
        self.list.borrow(py)
    }

    fn morph<'py>(&'py self, py: Python<'py>) -> MorphemeRef<'py> {
        let list = self.list(py);
        // workaround for self-referential structs
        let morph = unsafe { std::mem::transmute(list.internal(py).get(self.index)) };
        MorphemeRef { list, morph }
    }
}

#[pymethods]
impl PyMorpheme {
    fn __str__<'py>(&'py self, py: Python<'py>) -> PyObject {
        self.surface(py)
    }

    /// Returns the begin index of this in the input text
    #[pyo3(text_signature = "($self)")]
    fn begin(&self, py: Python) -> usize {
        // call codepoint version
        self.morph(py).begin_c()
    }

    /// Returns the end index of this in the input text
    #[pyo3(text_signature = "($self)")]
    fn end(&self, py: Python) -> usize {
        // call codepoint version
        self.morph(py).end_c()
    }

    /// Returns the surface
    #[pyo3(text_signature = "($self)")]
    fn surface<'py>(&'py self, py: Python<'py>) -> PyObject {
        self.morph(py).surface().deref().into_py(py)
    }

    /// Returns the part of speech
    #[pyo3(text_signature = "($self)")]
    fn part_of_speech<'py>(&'py self, py: Python<'py>) -> Py<PyTuple> {
        let pos_id = self.part_of_speech_id(py);
        self.list(py)
            .internal(py)
            .dict()
            .pos_of(pos_id)
            .clone_ref(py)
    }

    /// Returns the id of the part of speech in the dictionary
    #[pyo3(text_signature = "($self)")]
    pub fn part_of_speech_id(&self, py: Python) -> u16 {
        self.morph(py).part_of_speech_id()
    }

    /// Returns the dictionary form
    #[pyo3(text_signature = "($self)")]
    fn dictionary_form<'py>(&'py self, py: Python<'py>) -> PyObject {
        self.morph(py).get_word_info().dictionary_form().into_py(py)
    }

    /// Returns the normalized form
    #[pyo3(text_signature = "($self)")]
    fn normalized_form<'py>(&'py self, py: Python<'py>) -> PyObject {
        self.morph(py).get_word_info().normalized_form().into_py(py)
    }

    /// Returns the reading form
    #[pyo3(text_signature = "($self)")]
    fn reading_form<'py>(&'py self, py: Python<'py>) -> PyObject {
        self.morph(py).get_word_info().reading_form().into_py(py)
    }

    /// Returns a list of morphemes splitting itself with given split mode
    #[pyo3(
        text_signature = "($self, mode: sudachipy.SplitMode, out = None) -> sudachipy.MorphemeList"
    )]
    fn split<'py>(
        &'py self,
        py: Python<'py>,
        mode: PySplitMode,
        out: Option<&'py PyCell<PyMorphemeListWrapper>>,
        add_single: Option<bool>,
    ) -> PyResult<&'py PyCell<PyMorphemeListWrapper>> {
        let list = self.list(py);

        let out_cell = match out {
            None => {
                let list = list.empty_clone(py);
                PyCell::new(py, list)?
            }
            Some(r) => r,
        };

        let mut borrow = out_cell.try_borrow_mut();
        let out_ref = match borrow {
            Ok(ref mut v) => v.internal_mut(py),
            Err(_) => return Err(PyException::new_err("out was used twice")),
        };

        out_ref.clear();
        let splitted = list
            .internal(py)
            .split_into(mode.into(), self.index, out_ref)
            .map_err(|e| {
                PyException::new_err(format!("Error while splitting morpheme: {}", e.to_string()))
            })?;

        if add_single.unwrap_or(true) && !splitted {
            list.internal(py)
                .copy_slice(self.index, self.index + 1, out_ref);
        }

        Ok(out_cell)
    }

    /// Returns whether if this is out of vocabulary word
    #[pyo3(text_signature = "($self)")]
    fn is_oov(&self, py: Python) -> bool {
        self.morph(py).is_oov()
    }

    /// Returns word id of this word in the dictionary
    #[pyo3(text_signature = "($self)")]
    fn word_id(&self, py: Python) -> u32 {
        self.morph(py).word_id().as_raw()
    }

    /// Returns the dictionary id which this word belongs
    #[pyo3(text_signature = "($self)")]
    fn dictionary_id(&self, py: Python) -> i32 {
        let word_id = self.morph(py).word_id();
        if word_id.is_oov() {
            -1
        } else {
            word_id.dic() as i32
        }
    }

    /// Returns the list of synonym group ids
    #[pyo3(text_signature = "($self)")]
    fn synonym_group_ids<'py>(&'py self, py: Python<'py>) -> &'py PyList {
        let mref = self.morph(py);
        let ids = mref.get_word_info().synonym_group_ids();
        PyList::new(py, ids)
    }

    /// Returns the word info
    #[pyo3(text_signature = "($self) -> sudachipy.WordInfo")]
    fn get_word_info(&self, py: Python) -> PyResult<PyWordInfo> {
        let cat = PyModule::import(py, "builtins")?.getattr("DeprecationWarning")?;
        PyErr::warn(py, cat, "Users should not touch the raw WordInfo.", 1)?;

        Ok(self.morph(py).get_word_info().clone().into())
    }

    /// Returns morpheme length in codepoints
    pub fn __len__(&self, py: Python) -> usize {
        let m = self.morph(py);
        m.end_c() - m.begin_c()
    }
}
