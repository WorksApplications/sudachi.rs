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

use sudachi::analysis::stateless_tokenizer::StatelessTokenizer;
use sudachi::dic::dictionary::JapaneseDictionary;
use sudachi::prelude::*;

use crate::morpheme::PyMorphemeListWrapper;

/// Unit to split text
///
/// A == short mode
/// B == middle mode
/// C == long mode
//
// This implementation is a workaround. Waiting for the pyo3 enum feature.
// ref: [PyO3 issue #834](https://github.com/PyO3/pyo3/issues/834).
#[pyclass(module = "sudachi.tokenizer", name = "SplitMode")]
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

#[pyclass(module = "sudachi.tokenizer", name = "Tokenizer")]
pub struct PyTokenizer {
    tokenizer: StatelessTokenizer<Arc<JapaneseDictionary>>,
    mode: Mode,
}

impl PyTokenizer {
    pub fn new(tokenizer: StatelessTokenizer<Arc<JapaneseDictionary>>, mode: Mode) -> Self {
        Self { tokenizer, mode }
    }
}

#[pymethods]
impl PyTokenizer {
    #[classattr]
    #[allow(non_snake_case)]
    fn SplitMode() -> PySplitMode {
        PySplitMode::C
    }

    /// Break text into morphemes
    #[pyo3(text_signature = "($self, text, /, mode, enable_debug)")]
    #[args(text, mode = "None", enable_debug = "None")]
    fn tokenize(
        &self,
        text: &str,
        mode: Option<PySplitMode>,
        enable_debug: Option<bool>, // want to take logger instead of debug flag
    ) -> PyResult<PyMorphemeListWrapper> {
        let mode: Mode = match mode {
            Some(m) => m.into(),
            None => self.mode,
        };

        let morphemes = self
            .tokenizer
            .tokenize(text, mode, enable_debug.unwrap_or(false))
            .map_err(|e| {
                PyException::new_err(format!("Error while tokenization: {}", e.to_string()))
            })?
            .into();

        Ok(morphemes)
    }
}
