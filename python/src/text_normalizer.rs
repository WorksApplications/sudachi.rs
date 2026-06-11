/*
 *  Copyright (c) 2026 Works Applications Co., Ltd.
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

use sudachi::text_normalizer::TextNormalizer;

use crate::dictionary::PyDictionary;
use crate::errors;

/// A text normalizer.
///
/// This applies input-text plugins to raw input text. It does not perform
/// morphological analysis or return morpheme normalized forms.
///
/// Create using ``Dictionary.text_normalizer()`` or by passing a
/// ``Dictionary`` to this class. Without a dictionary, this uses the default
/// input-text normalization.
#[pyclass(module = "sudachipy", name = "TextNormalizer")]
pub struct PyTextNormalizer {
    normalizer: TextNormalizer<'static>,
}

impl PyTextNormalizer {
    pub(crate) fn from_dictionary(dictionary: &PyDictionary) -> Self {
        Self {
            normalizer: TextNormalizer::from_shared_dictionary(dictionary.data()),
        }
    }
}

#[pymethods]
impl PyTextNormalizer {
    /// Creates a text normalizer.
    ///
    /// When dictionary is provided, this applies the same input-text plugins
    /// used by that dictionary before tokenization. Without a dictionary, this
    /// applies the default input-text normalization.
    #[new]
    #[pyo3(
        signature = (dictionary=None),
        text_signature = "(dictionary=None) -> TextNormalizer"
    )]
    fn new(dictionary: Option<PyRef<'_, PyDictionary>>) -> PyResult<PyTextNormalizer> {
        match dictionary {
            Some(dictionary) => Ok(PyTextNormalizer::from_dictionary(&dictionary)),
            None => Ok(Self {
                normalizer: errors::wrap(TextNormalizer::default())?,
            }),
        }
    }

    /// Normalize text using input-text plugins.
    ///
    /// This normalizes tokenizer input text, not the dictionary-normalized form
    /// returned by ``Morpheme.normalized_form()``.
    ///
    /// :param text: text to normalize.
    /// :type text: str
    #[pyo3(text_signature = "(self, /, text: str) -> str")]
    fn normalize(&mut self, py: Python<'_>, text: &str) -> PyResult<String> {
        errors::wrap_ctx(
            py.detach(|| self.normalizer.normalize(text)),
            "Error during text normalization",
        )
    }
}
