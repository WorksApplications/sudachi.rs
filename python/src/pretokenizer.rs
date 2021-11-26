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
use sudachi::analysis::node::LatticeNode;
use sudachi::analysis::stateful_tokenizer::StatefulTokenizer;
use sudachi::dic::subset::InfoSubset;
use sudachi::prelude::{Mode, MorphemeList};
use thread_local::ThreadLocal;

/// This struct perform actual tokenization
/// There should be at most one instance per thread of execution
struct PerThreadPreTokenizer {
    tokenizer: StatefulTokenizer<Arc<PyDicData>>,
    morphemes: MorphemeList<Arc<PyDicData>>,
}

impl PerThreadPreTokenizer {
    pub fn new(dict: &Arc<PyDicData>, mode: Mode, subset: InfoSubset) -> Self {
        let mut tok = StatefulTokenizer::new(dict.clone(), mode);
        tok.set_subset(subset);
        Self {
            tokenizer: tok,
            morphemes: MorphemeList::empty(dict.clone()),
        }
    }

    pub fn tokenize(&mut self, data: &str) -> PyResult<()> {
        self.tokenizer.reset().push_str(data);
        wrap(self.tokenizer.do_tokenize())?;
        self.morphemes.collect_results(&mut self.tokenizer).unwrap();
        Ok(())
    }

    pub fn result(&self) -> &MorphemeList<Arc<PyDicData>> {
        &self.morphemes
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
        let cell = self.tokenizer_cell().borrow();
        let morphs = cell.result();
        match self.handler.as_ref() {
            None => {
                let result = PyList::empty(py);
                for idx in 0..morphs.len() {
                    let node = morphs.get_node(idx);
                    let slice = PySlice::new(py, node.begin() as isize, node.end() as isize, 1);
                    let args = PyTuple::new(py, [slice]);
                    let substring = string.call_method1("slice", args)?;
                    result.append(substring)?;
                }
                Ok(result)
            }
            Some(h) => {
                let mlist = PyMorphemeListWrapper::from(morphs.clone());
                let mlist = PyCell::new(py, mlist)?;
                let args = PyTuple::new(py, &[index, string, mlist]);
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
