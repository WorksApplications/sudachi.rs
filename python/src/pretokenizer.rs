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
use crate::errors::wrap;
use crate::morpheme::PyMorphemeListWrapper;
use pyo3::prelude::*;
use pyo3::types::{PyList, PySlice, PyTuple};
use std::cell::RefCell;
use std::sync::Arc;

use sudachi::analysis::stateful_tokenizer::StatefulTokenizer;
use sudachi::dic::subset::InfoSubset;
use sudachi::prelude::Mode;
use thread_local::ThreadLocal;

/// This struct perform actual tokenization
/// There should be at most one instance per thread of execution
struct PerThreadPreTokenizer {
    tokenizer: StatefulTokenizer<Arc<PyDicData>>,
    morphemes: Option<Py<PyMorphemeListWrapper>>,
}

impl PerThreadPreTokenizer {
    pub fn new(dict: &Arc<PyDicData>, mode: Mode, subset: InfoSubset) -> Self {
        let mut tok = StatefulTokenizer::new(dict.clone(), mode);
        tok.set_subset(subset);
        Self {
            tokenizer: tok,
            morphemes: None,
        }
    }

    pub fn tokenize(&mut self, data: &str) -> PyResult<()> {
        self.tokenizer.reset().push_str(data);
        wrap(self.tokenizer.do_tokenize())?;
        Ok(())
    }

    pub fn collect_results(&mut self, py: Python) -> PyResult<()> {
        let mut mlist = match self.morphemes.as_mut() {
            None => {
                self.morphemes = Some(Py::new(
                    py,
                    PyMorphemeListWrapper::new(self.tokenizer.dict_clone()),
                )?);
                self.morphemes.as_mut().unwrap().borrow_mut(py)
            }
            Some(ms) => ms.borrow_mut(py),
        };
        mlist
            .internal_mut(py)
            .collect_results(&mut self.tokenizer)
            .unwrap();
        Ok(())
    }

    pub fn result(&self) -> &Py<PyMorphemeListWrapper> {
        self.morphemes.as_ref().unwrap()
    }
}

/// Binding for the Tokenizer, which handles threading for tokenization
///
/// We use ThreadLocal for storing actual tokenizers
#[pyclass(module = "sudachipy.pretokenizer", name = "SudachiPreTokenizer")]
pub struct PyPretokenizer {
    dict: Arc<PyDicData>,
    mode: Mode,
    subset: InfoSubset,
    tokenizers: ThreadLocal<RefCell<PerThreadPreTokenizer>>,
    handler: Option<Py<PyAny>>,
}

impl PyPretokenizer {
    pub(crate) fn new(
        dict: Arc<PyDicData>,
        mode: Mode,
        subset: InfoSubset,
        handler: Option<Py<PyAny>>,
    ) -> PyPretokenizer {
        Self {
            dict,
            mode,
            subset,
            tokenizers: ThreadLocal::new(),
            handler,
        }
    }

    fn tokenizer_cell(&self) -> &RefCell<PerThreadPreTokenizer> {
        let tok = self.tokenizers.get_or(|| {
            RefCell::new(PerThreadPreTokenizer::new(
                &self.dict,
                self.mode,
                self.subset,
            ))
        });

        tok
    }
}

#[pymethods]
impl PyPretokenizer {
    /// Perform a tokenization for a sentence (passed as string)
    ///
    /// Implementation uses Sudachi to perform the analysis, then uses slice method
    /// of the passed parameter to create output data
    pub fn __call__<'p>(
        &'p self,
        py: Python<'p>,
        index: &'p PyAny,
        string: &'p PyAny,
    ) -> PyResult<&'p PyAny> {
        let input_data = string.str()?.to_str()?;
        // tokenization itself should work without GIL, we have thread-local tokenizers here
        py.allow_threads(|| self.tokenizer_cell().borrow_mut().tokenize(input_data))?;
        // then prepare results with GIL
        self.tokenizer_cell().borrow_mut().collect_results(py)?;
        let cell = self.tokenizer_cell().borrow();
        let morphs = cell.result();
        match self.handler.as_ref() {
            None => {
                let result = PyList::empty(py);
                let py_ref = morphs.borrow(py);
                let morphs = py_ref.internal(py);
                for idx in 0..morphs.len() {
                    let node = morphs.get(idx);
                    let slice = PySlice::new(py, node.begin_c() as isize, node.end_c() as isize, 1);
                    let args = PyTuple::new(py, [slice]);
                    let substring = string.call_method1("slice", args)?;
                    result.append(substring)?;
                }
                Ok(result)
            }
            Some(h) => {
                let mrp: &PyAny = morphs.as_ref(py);
                let args = PyTuple::new(py, &[index, string, mrp]);
                h.as_ref(py).call1(args)
            }
        }
    }

    /// Entry function for tokenization
    pub fn pre_tokenize<'p>(
        self_: &'p PyCell<Self>,
        py: Python<'p>,
        data: &'p PyAny,
    ) -> PyResult<&'p PyAny> {
        data.call_method1("split", PyTuple::new(py, [self_]))
    }
}
