/*
 *  Copyright (c) 2021-2024 Works Applications Co., Ltd.
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

use std::cell::RefCell;
use std::sync::Arc;

use pyo3::intern;
use pyo3::prelude::*;
use pyo3::sync::PyOnceLock;
use pyo3::types::{PyList, PySlice, PyType};
use thread_local::ThreadLocal;

use sudachi::analysis::stateful_tokenizer::StatefulTokenizer;
use sudachi::dic::subset::InfoSubset;
use sudachi::prelude::Mode;

use crate::dictionary::PyDicData;
use crate::errors;
use crate::morpheme::{PyMorphemeList, PyMorphemeListWrapper};
use crate::projection::{MorphemeProjection, PyProjector};

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
        errors::wrap(self.tokenizer.do_tokenize())?;
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

/// Binding for the Tokenizer, which handles threading for tokenization.
///
/// Create using Dictionary.pre_tokenizer method.
/// We use ThreadLocal for storing actual tokenizers.
#[pyclass(module = "sudachipy.pretokenizer", name = "SudachiPreTokenizer")]
pub struct PyPretokenizer {
    dict: Arc<PyDicData>,
    mode: Mode,
    subset: InfoSubset,
    tokenizers: ThreadLocal<RefCell<PerThreadPreTokenizer>>,
    handler: Option<Py<PyAny>>,
    projection: PyProjector,
}

impl PyPretokenizer {
    pub(crate) fn new(
        dict: Arc<PyDicData>,
        mode: Mode,
        subset: InfoSubset,
        handler: Option<Py<PyAny>>,
        projection: PyProjector,
    ) -> PyPretokenizer {
        Self {
            dict,
            mode,
            subset,
            tokenizers: ThreadLocal::new(),
            handler,
            projection,
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
    pub fn __call__<'py>(
        &'py self,
        py: Python<'py>,
        index: &Bound<'py, PyAny>,
        string: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let pystr = string.str()?;
        let input_data = pystr.to_cow()?.into_owned();
        // tokenization itself should work without GIL, we have thread-local tokenizers here
        py.detach(|| self.tokenizer_cell().borrow_mut().tokenize(&input_data))?;
        // then prepare results with GIL
        self.tokenizer_cell().borrow_mut().collect_results(py)?;
        let cell = self.tokenizer_cell().borrow();
        let morphs = cell.result();
        match self.handler.as_ref() {
            None => {
                let py_ref = morphs.borrow(py);
                let morphs = py_ref.internal(py);
                match self.projection.as_deref() {
                    None => make_result_for_surface(py, morphs, string).map(|bl| bl.into_any()),
                    Some(p) => make_result_for_projection(py, morphs, p).map(|bl| bl.into_any()),
                }
            }
            Some(h) => {
                let mrp: &Bound<PyAny> = morphs.bind(py);
                h.bind(py).call1((index, string, mrp))
            }
        }
    }

    /// Entry function for tokenization
    pub fn pre_tokenize<'py>(
        self_: Bound<'py, Self>,
        py: Python<'py>,
        data: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        data.call_method1(intern!(py, "split"), (self_,))
    }
}

fn make_result_for_surface<'py>(
    py: Python<'py>,
    morphs: &PyMorphemeList,
    string: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyList>> {
    let result = PyList::empty(py);
    for idx in 0..morphs.len() {
        let node = morphs.get(idx);
        let slice = PySlice::new(py, node.begin_c() as isize, node.end_c() as isize, 1);
        let substring = string.call_method1(intern!(py, "slice"), (slice,))?;
        result.append(substring)?;
    }
    Ok(result)
}

fn make_result_for_projection<'py>(
    py: Python<'py>,
    morphs: &PyMorphemeList,
    proj: &dyn MorphemeProjection,
) -> PyResult<Bound<'py, PyList>> {
    let result = PyList::empty(py);
    let nstring = {
        static NORMALIZED_STRING: PyOnceLock<Py<PyType>> = PyOnceLock::new();
        NORMALIZED_STRING.get_or_try_init(py, || -> PyResult<Py<PyType>> {
            let ns = py.import("tokenizers")?.getattr("NormalizedString")?;
            let tpe = ns.cast::<PyType>()?;
            Ok(tpe.clone().unbind())
        })?
    };
    for idx in 0..morphs.len() {
        let node = morphs.get(idx);
        let value = proj.project(&node, py);
        let substring = nstring.call1(py, (value,))?;
        result.append(substring)?;
    }
    Ok(result)
}
