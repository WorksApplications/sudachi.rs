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

use sudachi::analysis::morpheme_list::{Morpheme, MorphemeList};
use sudachi::analysis::node::Node;
use sudachi::dic::dictionary::JapaneseDictionary;
use sudachi::dic::lexicon::word_infos::WordInfo;
use sudachi::dic::lexicon_set::LexiconSet;
use sudachi::prelude::*;

use crate::tokenizer::PySplitMode;

#[pyclass]
pub struct PyMorphemeListWrapper {
    inner: Arc<PyMorphemeList>,
}

pub struct PyMorphemeList {
    dict: Arc<JapaneseDictionary>,
    input_text: String,
    path: Vec<Node>,
}

impl From<MorphemeList<'_, Arc<JapaneseDictionary>>> for PyMorphemeListWrapper {
    fn from(morpheme_list: MorphemeList<Arc<JapaneseDictionary>>) -> Self {
        Self {
            inner: Arc::new(PyMorphemeList {
                dict: morpheme_list.dict,
                input_text: morpheme_list.input_text.to_string(),
                path: morpheme_list.path,
            }),
        }
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

#[pyclass]
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
            // word_info: None,
        };

        slf.index += 1;
        Some(morpheme)
    }
}

#[pyclass]
pub struct PyMorpheme {
    list: Arc<PyMorphemeList>,
    index: usize,
}
