/*
 * Copyright (c) 2021 Works Applications Co., Ltd.
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use std::path::PathBuf;
use std::sync::Arc;

use pyo3::exceptions::PyException;
use pyo3::prelude::*;

use sudachi::analysis::stateless_tokenizer::StatelessTokenizer;
use sudachi::config::Config;
use sudachi::dic::dictionary::JapaneseDictionary;

use crate::tokenizer::{PySplitMode, PyTokenizer};

#[pyclass(module = "sudachi.dictionary", name = "Dictionary")]
#[pyo3(text_signature = "(config_path, resource_dir)")]
#[derive(Clone)]
pub struct PyDictionary {
    pub(super) dictionary: Arc<JapaneseDictionary>,
}

#[pymethods]
impl PyDictionary {
    /// Creates a sudachi dictionary
    #[new]
    #[args(config_path = "None", resource_dir = "None")]
    fn new(config_path: Option<PathBuf>, resource_dir: Option<PathBuf>) -> PyResult<Self> {
        let config = Config::new(config_path, resource_dir, None).map_err(|e| {
            PyException::new_err(format!("Error loading config: {}", e.to_string()))
        })?;

        let dictionary = Arc::new(JapaneseDictionary::from_cfg(&config).map_err(|e| {
            PyException::new_err(format!(
                "Error while constructing dictionary: {}",
                e.to_string()
            ))
        })?);

        Ok(Self { dictionary })
    }

    /// Creates a sudachi tokenizer
    #[pyo3(text_signature = "($self, mode)")]
    #[args(mode = "None")]
    fn create(&self, mode: Option<PySplitMode>) -> PyTokenizer {
        let tokenizer = StatelessTokenizer::new(self.dictionary.clone());
        let mode = mode.unwrap_or(PySplitMode::C).into();

        PyTokenizer::new(tokenizer, mode)
    }
}
