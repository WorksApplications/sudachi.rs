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

use std::ops::DerefMut;
use std::str::FromStr;
use std::sync::Arc;

use pyo3::prelude::*;

use sudachi::analysis::stateful_tokenizer::StatefulTokenizer;

use sudachi::dic::subset::InfoSubset;
use sudachi::prelude::*;

use crate::dictionary::{extract_mode, PyDicData};
use crate::errors::SudachiError as SudachiPyErr;
use crate::morpheme::{PyMorphemeListWrapper, PyProjector};

/// Unit to split text
///
/// A == short mode
///
/// B == middle mode
///
/// C == long mode
//
#[pyclass(module = "sudachipy.tokenizer", name = "SplitMode", frozen)]
#[derive(Clone, PartialEq, Eq, Copy, Debug)]
#[repr(u8)]
pub enum PySplitMode {
    A,
    B,
    C,
}

impl From<PySplitMode> for Mode {
    fn from(mode: PySplitMode) -> Self {
        match mode {
            PySplitMode::A => Mode::A,
            PySplitMode::B => Mode::B,
            PySplitMode::C => Mode::C,
        }
    }
}

impl From<Mode> for PySplitMode {
    fn from(value: Mode) -> Self {
        match value {
            Mode::A => PySplitMode::A,
            Mode::B => PySplitMode::B,
            Mode::C => PySplitMode::C,
        }
    }
}

#[pymethods]
impl PySplitMode {
    #[new]
    fn new(mode: Option<&str>) -> PyResult<PySplitMode> {
        let mode = match mode {
            Some(m) => m,
            None => return Ok(PySplitMode::C),
        };

        match Mode::from_str(mode) {
            Ok(m) => Ok(m.into()),
            Err(e) => Err(SudachiPyErr::new_err(e.to_string())),
        }
    }
}

/// Sudachi Tokenizer, Python version
#[pyclass(module = "sudachipy.tokenizer", name = "Tokenizer")]
pub(crate) struct PyTokenizer {
    tokenizer: StatefulTokenizer<Arc<PyDicData>>,
    projection: PyProjector,
}

impl PyTokenizer {
    pub(crate) fn new(
        dict: Arc<PyDicData>,
        mode: Mode,
        fields: InfoSubset,
        projection: PyProjector,
    ) -> Self {
        let mut tok = Self {
            tokenizer: StatefulTokenizer::new(dict, mode),
            projection,
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
        text_signature = "($self, text: str, mode = None, logger = None, out = None) -> sudachipy.MorphemeList",
        signature = (text, mode = None, logger = None, out = None)
    )]
    #[allow(unused_variables)]
    fn tokenize<'py>(
        &'py mut self,
        py: Python<'py>,
        text: &'py str,
        mode: Option<&PyAny>,
        logger: Option<PyObject>,
        out: Option<&'py PyCell<PyMorphemeListWrapper>>,
    ) -> PyResult<&'py PyCell<PyMorphemeListWrapper>> {
        // restore default mode on scope exit
        let mode = match mode {
            None => None,
            Some(m) => Some(extract_mode(py, m)?),
        };
        let default_mode = mode.map(|m| self.tokenizer.set_mode(m.into()));
        let mut tokenizer = scopeguard::guard(&mut self.tokenizer, |t| {
            default_mode.map(|m| t.set_mode(m));
        });

        // analysis can be done without GIL
        let err = py.allow_threads(|| {
            tokenizer.reset().push_str(text);
            tokenizer.do_tokenize()
        });

        err.map_err(|e| SudachiPyErr::new_err(format!("Tokenization error: {}", e.to_string())))?;

        let out_list = match out {
            None => {
                let dict = tokenizer.dict_clone();
                let morphemes = MorphemeList::empty(dict);
                let wrapper =
                    PyMorphemeListWrapper::from_components(morphemes, self.projection.clone());
                PyCell::new(py, wrapper)?
            }
            Some(list) => list,
        };

        let mut borrow = out_list.try_borrow_mut();
        let morphemes = match borrow {
            Ok(ref mut ms) => ms.internal_mut(py),
            Err(e) => return Err(SudachiPyErr::new_err("out was used twice at the same time")),
        };

        morphemes
            .collect_results(tokenizer.deref_mut())
            .map_err(|e| SudachiPyErr::new_err(format!("Tokenization error: {}", e.to_string())))?;

        Ok(out_list)
    }

    #[getter]
    fn mode(&self) -> PySplitMode {
        self.tokenizer.mode().into()
    }
}
