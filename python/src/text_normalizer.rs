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

use std::sync::Arc;

use pyo3::prelude::*;

use sudachi::dic::DictionaryAccess;
use sudachi::error::SudachiResult;
use sudachi::input_text::InputBuffer;

use crate::dictionary::{PyDicData, PyDictionary};
use crate::errors;

/// A text normalizer.
///
/// This applies dictionary input-text plugins to raw input text. It does not
/// perform morphological analysis or return morpheme normalized forms.
///
/// Create using ``Dictionary.text_normalizer()`` or by passing a
/// ``Dictionary`` to this class.
#[pyclass(module = "sudachipy", name = "TextNormalizer")]
pub struct PyTextNormalizer {
    dict: Arc<PyDicData>,
}

impl PyTextNormalizer {
    pub(crate) fn from_dictionary(dict: Arc<PyDicData>) -> Self {
        Self { dict }
    }
}

#[pymethods]
impl PyTextNormalizer {
    /// Creates a text normalizer from a dictionary.
    ///
    /// The normalizer applies the same input-text plugins that the dictionary
    /// uses before tokenization. The normalizer can keep normalizing text after
    /// the source dictionary is closed.
    #[new]
    #[pyo3(text_signature = "(dictionary) -> TextNormalizer")]
    fn new(dictionary: PyRef<'_, PyDictionary>) -> PyTextNormalizer {
        PyTextNormalizer::from_dictionary(dictionary.data())
    }

    /// Normalize text using dictionary input-text plugins.
    ///
    /// This normalizes tokenizer input text, not the dictionary-normalized form
    /// returned by ``Morpheme.normalized_form()``.
    ///
    /// :param text: text to normalize.
    /// :type text: str
    #[pyo3(text_signature = "(self, /, text: str) -> str")]
    fn normalize(&self, py: Python<'_>, text: &str) -> PyResult<String> {
        errors::wrap_ctx(
            py.detach(|| normalize_with_dictionary(&self.dict, text)),
            "Error during text normalization",
        )
    }
}

fn normalize_with_dictionary(dict: &PyDicData, text: &str) -> SudachiResult<String> {
    let mut buffer = InputBuffer::new();
    buffer.reset().push_str(text);
    buffer.start_build()?;
    for plugin in dict.input_text_plugins() {
        plugin.rewrite(&mut buffer)?;
    }
    Ok(buffer.current().to_owned())
}
