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

use pyo3::exceptions::PyException;
use pyo3::prelude::*;

use sudachi::analysis::stateful_tokenizer::StatefulTokenizer;

use sudachi::dic::subset::InfoSubset;
use sudachi::prelude::*;

use crate::dictionary::PyDicData;
use crate::morpheme::PyMorphemeListWrapper;

/// Unit to split text
///
/// A == short mode
///
/// B == middle mode
///
/// C == long mode
//
// This implementation is a workaround. Waiting for the pyo3 enum feature.
// ref: [PyO3 issue #834](https://github.com/PyO3/pyo3/issues/834).
#[pyclass(module = "sudachipy.tokenizer", name = "SplitMode")]
#[derive(Clone, PartialEq, Eq)]
#[repr(transparent)]
pub struct PySplitMode {
    mode: u8,
}

#[pymethods]
impl PySplitMode {
    #[classattr]
    pub const A: Self = Self { mode: 0 };

    #[classattr]
    pub const B: Self = Self { mode: 1 };

    #[classattr]
    pub const C: Self = Self { mode: 2 };
}

impl From<PySplitMode> for Mode {
    fn from(mode: PySplitMode) -> Self {
        match mode {
            PySplitMode::A => Mode::A,
            PySplitMode::B => Mode::B,
            _ => Mode::C,
        }
    }
}

/// Sudachi Tokenizer, Python version
#[pyclass(module = "sudachipy.tokenizer", name = "Tokenizer")]
pub struct PyTokenizer {
    tokenizer: StatefulTokenizer<Arc<PyDicData>>,
}

impl PyTokenizer {
    pub(crate) fn new(dict: Arc<PyDicData>, mode: Mode, fields: InfoSubset) -> Self {
        let mut tok = Self {
            tokenizer: StatefulTokenizer::new(dict, mode),
        };
        tok.tokenizer.set_subset(fields);
        tok
    }
}

#[pymethods]
impl PyTokenizer {
    #[classattr]
    #[allow(non_snake_case)]
    fn SplitMode() -> PySplitMode {
        PySplitMode::C
    }

    /// Break text into morphemes.
    ///
    /// SudachiPy 0.5.* had logger parameter, it is accepted, but ignored.
    ///
    /// :param text: text to analyze
    /// :param mode: analysis mode.
    ///    This parameter is deprecated.
    ///    Pass the analysis mode at the Tokenizer creation time and create different tokenizers for different modes.
    ///    If you need multi-level splitting, prefer using :py:meth:`Morpheme.split` method instead.
    /// :param out: tokenization results will be written into this MorphemeList, a new one will be created instead.
    ///    See https://worksapplications.github.io/sudachi.rs/python/topics/out_param.html for details.
    /// :type text: str
    /// :type mode: sudachipy.SplitMode
    /// :type out: sudachipy.MorphemeList
    #[pyo3(
        text_signature = "($self, text: str, mode: SplitMode = None, logger = None, out = None) -> sudachipy.MorphemeList"
    )]
    #[args(text, mode = "None", logger = "None")]
    #[allow(unused_variables)]
    fn tokenize<'py>(
        &'py mut self,
        py: Python<'py>,
        text: &'py str,
        mode: Option<PySplitMode>,
        logger: Option<PyObject>,
        out: Option<&'py PyCell<PyMorphemeListWrapper>>,
    ) -> PyResult<&'py PyCell<PyMorphemeListWrapper>> {
        // keep default mode to restore later
        let default_mode = mode.map(|m| self.tokenizer.set_mode(m.into()));

        self.tokenizer.reset().push_str(text);
        self.tokenizer
            .do_tokenize()
            .map_err(|e| PyException::new_err(format!("Tokenization error: {}", e.to_string())))?;

        let out_list = match out {
            None => {
                let morphemes = MorphemeList::empty(self.tokenizer.dict_clone());
                let wrapper = PyMorphemeListWrapper::from(morphemes);
                PyCell::new(py, wrapper)?
            }
            Some(list) => list,
        };

        let mut borrow = out_list.try_borrow_mut();
        let morphemes = match borrow {
            Ok(ref mut ms) => ms.internal_mut(py),
            Err(e) => return Err(PyException::new_err("out was used twice at the same time")),
        };

        morphemes
            .collect_results(&mut self.tokenizer)
            .map_err(|e| PyException::new_err(format!("Tokenization error: {}", e.to_string())))?;

        // restore default mode
        default_mode.map(|m| self.tokenizer.set_mode(m));

        Ok(out_list)
    }
}
