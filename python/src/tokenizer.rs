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

use std::ops::DerefMut;
use std::str::FromStr;
use std::sync::Arc;

use pyo3::prelude::*;

use sudachi::analysis::stateful_tokenizer::StatefulTokenizer;
use sudachi::dic::subset::InfoSubset;
use sudachi::prelude::*;

use crate::dictionary::{extract_mode, PyDicData};
use crate::errors;
use crate::morpheme::PyMorphemeListWrapper;
use crate::projection::PyProjector;

/// Unit to split text.
///
/// A == short mode
///
/// B == middle mode
///
/// C == long mode
///
/// :param mode: string representation of the split mode. One of [A,B,C] in captital or lower case.
///     If None, returns SplitMode.C.
///
/// :type mode: str | None
#[pyclass(
    module = "sudachipy.tokenizer",
    name = "SplitMode",
    eq,
    eq_int,
    frozen,
    from_py_object
)]
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
    /// Creates a split mode from a string value.
    ///
    /// :param mode: string representation of the split mode. One of [A,B,C] in captital or lower case.
    ///     If None, returns SplitMode.C.
    ///
    /// :type mode: str | None
    #[new]
    #[pyo3(
        signature = (mode=None),
        text_signature = "(mode=None) -> SplitMode"
    )]
    fn new(mode: Option<&str>) -> PyResult<PySplitMode> {
        let mode = match mode {
            Some(m) => m,
            None => return Ok(PySplitMode::C),
        };
        errors::wrap(Mode::from_str(mode).map(|m| m.into()))
    }
}

/// A sudachi tokenizer
///
/// Create using Dictionary.create method.
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
    /// :param text: text to analyze.
    /// :param mode: analysis mode.
    ///    This parameter is deprecated.
    ///    Pass the analysis mode at the Tokenizer creation time and create different tokenizers for different modes.
    ///    If you need multi-level splitting, prefer using :py:meth:`Morpheme.split` method instead.
    /// :param logger: Arg for v0.5.* compatibility. Ignored.
    /// :param out: tokenization results will be written into this MorphemeList, a new one will be created instead.
    ///    See https://worksapplications.github.io/sudachi.rs/python/topics/out_param.html for details.
    ///
    /// A Tokenizer instance cannot be used concurrently from multiple threads.
    /// For parallel tokenization, share a Dictionary and create one Tokenizer
    /// per worker/thread. Concurrent calls raise sudachipy.errors.SudachiError.
    ///
    /// :type text: str
    /// :type mode: SplitMode | str | None
    /// :type out: MorphemeList
    #[pyo3(
        text_signature="(self, /, text: str, mode=None, logger=None, out=None) -> MorphemeList",
        signature=(text, mode=None, logger=None, out=None)
    )]
    #[allow(unused_variables)]
    fn tokenize<'py>(
        self_: &Bound<'py, Self>,
        py: Python<'py>,
        text: &'py str,
        mode: Option<&Bound<'py, PyAny>>,
        logger: Option<Py<PyAny>>,
        out: Option<Bound<'py, PyMorphemeListWrapper>>,
    ) -> PyResult<Bound<'py, PyMorphemeListWrapper>> {
        // restore default mode on scope exit
        let mode = match mode {
            None => None,
            Some(m) => Some(extract_mode(m)?),
        };

        let mut this = match self_.try_borrow_mut() {
            Ok(this) => this,
            Err(_) => {
                return errors::wrap(Err(
                    "Tokenizer is already in use. A Tokenizer instance cannot be used concurrently; create a separate Tokenizer per thread or guard calls externally",
                ))
            }
        };

        let projection = this.projection.clone();
        let default_mode = mode.map(|m| this.tokenizer.set_mode(m));
        let mut tokenizer = scopeguard::guard(&mut this.tokenizer, |t| {
            default_mode.map(|m| t.set_mode(m));
        });

        // analysis can be done without GIL
        errors::wrap_ctx(
            py.detach(|| {
                tokenizer.reset().push_str(text);
                tokenizer.do_tokenize()
            }),
            "Error during tokenization",
        )?;

        let out_list = match out {
            None => {
                let dict = tokenizer.dict_clone();
                let morphemes = MorphemeList::empty(dict);
                let wrapper = PyMorphemeListWrapper::from_components(morphemes, projection);
                Bound::new(py, wrapper)?
            }
            Some(list) => list,
        };

        let mut borrow = out_list.try_borrow_mut();
        let morphemes = match borrow {
            Ok(ref mut ms) => ms.internal_mut(py),
            Err(_) => return errors::wrap(Err("out was used twice at the same time")),
        };

        errors::wrap_ctx(
            morphemes.collect_results(tokenizer.deref_mut()),
            "Error during tokenization",
        )?;

        Ok(out_list)
    }

    /// SplitMode of the tokenizer.
    #[getter]
    fn mode(&self) -> PySplitMode {
        self.tokenizer.mode().into()
    }
}
