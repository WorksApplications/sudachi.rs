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

use crate::dictionary::PyDicData;
use crate::morpheme::PyMorpheme;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyIterator, PyTuple};
use std::sync::Arc;
use sudachi::analysis::stateless_tokenizer::DictionaryAccess;
use sudachi::pos::PosMatcher;

#[pyclass(name = "PosMatcher", module = "sudachipy")]
pub struct PyPosMatcher {
    matcher: PosMatcher,
    dic: Arc<PyDicData>,
}

impl PyPosMatcher {
    pub(crate) fn create<'py>(
        py: Python<'py>,
        dic: &'py Arc<PyDicData>,
        data: &'py PyAny,
    ) -> PyResult<PyPosMatcher> {
        if data.is_callable() {
            Self::create_from_fn(dic, data, py)
        } else {
            let iter = data.iter()?;
            Self::create_from_iter(dic, iter)
        }
    }

    fn create_from_fn(dic: &Arc<PyDicData>, func: &PyAny, py: Python) -> PyResult<Self> {
        let mut data = Vec::new();
        for (pos_id, pos) in dic.pos.iter().enumerate() {
            let args = PyTuple::new(py, &[pos]);
            if func.call1(args)?.cast_as::<PyBool>()?.is_true() {
                data.push(pos_id as u16);
            }
        }
        Ok(Self {
            matcher: PosMatcher::new(data),
            dic: dic.clone(),
        })
    }

    fn create_from_iter(dic: &Arc<PyDicData>, data: &PyIterator) -> PyResult<Self> {
        let mut result = Vec::new();
        for item in data {
            let item = item?.cast_as::<PyTuple>()?;
            Self::match_pos_elements(&mut result, dic.as_ref(), item)?;
        }
        Ok(Self {
            matcher: PosMatcher::new(result),
            dic: dic.clone(),
        })
    }

    fn match_pos_elements(data: &mut Vec<u16>, dic: &PyDicData, elem: &PyTuple) -> PyResult<()> {
        let start_len = data.len();

        let elen = elem.len();
        for (pos_id, pos) in dic.grammar().pos_list.iter().enumerate() {
            let check = |idx: usize| -> PyResult<bool> {
                let x = elem.get_item(idx)?;
                if x.is_none() {
                    return Ok(false);
                }
                Ok(x.str()?.to_str()? != pos[idx])
            };
            if elen > 0 && check(0)? {
                continue;
            }
            if elen > 1 && check(1)? {
                continue;
            }
            if elen > 2 && check(2)? {
                continue;
            }
            if elen > 3 && check(3)? {
                continue;
            }
            if elen > 4 && check(4)? {
                continue;
            }
            if elen > 5 && check(5)? {
                continue;
            }
            data.push(pos_id as u16);
        }

        if start_len == data.len() {
            Err(PyException::new_err(format!(
                "POS {:?} did not match any elements",
                elem.repr()?
            )))
        } else {
            Ok(())
        }
    }
}

#[pymethods]
impl PyPosMatcher {
    pub fn __call__<'py>(&'py self, py: Python<'py>, m: &'py PyMorpheme) -> bool {
        let pos_id = m.part_of_speech_id(py);
        self.matcher.matches_id(pos_id)
    }

    pub fn __str__(&self) -> String {
        format!("<PosMatcher:{} pos>", self.matcher.num_entries())
    }

    pub fn __iter__(&self) -> PyPosIter {
        PyPosIter::new(self.matcher.entries(), self.dic.clone())
    }

    pub fn __len__(&self) -> usize {
        self.matcher.num_entries()
    }
}

#[pyclass(name = "PosMatcherIterator", module = "sudachipy")]
pub struct PyPosIter {
    data: Vec<u16>,
    dic: Arc<PyDicData>,
    index: usize,
}

impl PyPosIter {
    fn new(data: impl Iterator<Item = u16>, dic: Arc<PyDicData>) -> Self {
        let mut result: Vec<u16> = data.collect();
        result.sort();
        Self {
            data: result,
            dic,
            index: 0,
        }
    }
}

#[pymethods]
impl PyPosIter {
    fn __iter__(slf: &PyCell<Self>) -> &PyCell<Self> {
        slf
    }

    fn __next__<'py>(&'py mut self, py: Python<'py>) -> Option<&'py PyTuple> {
        let idx = self.index;
        self.index += 1;
        if idx >= self.data.len() {
            return None;
        }
        let pos_id = self.data[idx];
        let pos = &self.dic.pos[pos_id as usize];
        Some(pos.as_ref(py))
    }
}
