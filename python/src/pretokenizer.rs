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

use pyo3::prelude::*;
use pyo3::types::{PyList, PySlice, PyString, PyTuple};
use pyo3::PyObjectProtocol;
use std::borrow::Borrow;
use std::cell::{RefCell, RefMut};
use std::sync::Arc;
use sudachi::analysis::node::LatticeNode;
use sudachi::analysis::stateful_tokenizer::StatefulTokenizer;
use sudachi::dic::dictionary::JapaneseDictionary;
use sudachi::prelude::{Mode, MorphemeList};
use thread_local::ThreadLocal;

struct PerThreadPreTokenizer {
    tokenizer: StatefulTokenizer<Arc<JapaneseDictionary>>,
    morphemes: MorphemeList<Arc<JapaneseDictionary>>,
}

impl PerThreadPreTokenizer {
    pub fn new(dict: &Arc<JapaneseDictionary>, mode: Mode) -> Self {
        Self {
            tokenizer: StatefulTokenizer::new(dict.clone(), mode),
            morphemes: MorphemeList::empty(dict.clone()),
        }
    }

    pub fn tokenize(&mut self, data: &str) -> PyResult<()> {
        self.tokenizer.reset().push_str(data);
        self.tokenizer.do_tokenize().map_err(|e| panic!("{:?}", e));
        self.morphemes.collect_results(&mut self.tokenizer).unwrap();
        Ok(())
    }

    pub fn result(&self) -> &MorphemeList<Arc<JapaneseDictionary>> {
        &self.morphemes
    }
}

#[pyclass(module = "sudachipy.pretokenizer", name = "SudachiPreTokenizer")]
pub struct PyPretokenizer {
    dict: Arc<JapaneseDictionary>,
    mode: Mode,
    tokenizers: ThreadLocal<RefCell<PerThreadPreTokenizer>>,
}

impl PyPretokenizer {
    pub fn new(dict: Arc<JapaneseDictionary>, mode: Mode) -> PyPretokenizer {
        Self {
            dict,
            mode,
            tokenizers: ThreadLocal::new(),
        }
    }

    fn tokenizer_cell(&self) -> &RefCell<PerThreadPreTokenizer> {
        let tok = self
            .tokenizers
            .get_or(|| RefCell::new(PerThreadPreTokenizer::new(&self.dict, Mode::C)));

        tok
    }
}

#[pymethods]
impl PyPretokenizer {
    pub fn __call__<'p>(
        &'p self,
        py: Python<'p>,
        _index: &'p PyAny,
        string: &'p PyAny,
    ) -> PyResult<&'p PyList> {
        let input_data = string.str()?.to_str()?;
        // tokenization itself should work without GIL, it is thread-safe
        py.allow_threads(|| self.tokenizer_cell().borrow_mut().tokenize(input_data))?;
        // then copy results again with GIL
        let cell = self.tokenizer_cell().borrow();
        let morphs = cell.result();
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

    pub fn pre_tokenize<'p>(
        self_: &'p PyCell<Self>,
        py: Python<'p>,
        data: &'p PyAny,
    ) -> PyResult<&'p PyAny> {
        data.call_method1("split", PyTuple::new(py, [self_]))
    }

    // RefCell is not Sync, so can't use iter() :(
    // Python's GIL makes it OK to use mut here though
    pub fn __repr__<'p>(&'p mut self, _py: Python<'p>) -> String {
        let iter = self.tokenizers.iter_mut();
        let cnt = iter.count();
        format!("SudachiPreTokenizer(mode={}, instances={})", self.mode, cnt)
    }
}
