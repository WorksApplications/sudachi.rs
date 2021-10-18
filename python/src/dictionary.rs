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

use std::path::PathBuf;
use std::sync::Arc;

use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use pyo3::types::PyString;

use sudachi::analysis::stateless_tokenizer::StatelessTokenizer;
use sudachi::config::Config;
use sudachi::dic::dictionary::JapaneseDictionary;

use crate::tokenizer::{PySplitMode, PyTokenizer};

#[pyclass(module = "sudachi.dictionary", name = "Dictionary")]
#[pyo3(text_signature = "(config_path, resource_dir)")]
#[derive(Clone)]
#[repr(transparent)]
pub struct PyDictionary {
    pub(super) dictionary: Option<Arc<JapaneseDictionary>>,
}

#[pymethods]
impl PyDictionary {
    /// Creates a sudachi dictionary
    #[new]
    #[args(config_path = "None", resource_dir = "None", dict_type = "None")]
    fn new(
        py: Python,
        config_path: Option<PathBuf>,
        resource_dir: Option<PathBuf>,
        dict_type: Option<&str>,
    ) -> PyResult<Self> {
        let config_path = match config_path {
            None => Some(get_default_setting_path(py)?),
            Some(v) => Some(v),
        };
        let resource_dir = match resource_dir {
            None => Some(get_default_resource_dir(py)?),
            Some(v) => Some(v),
        };
        let dict_path = match dict_type {
            None => None,
            Some(dt) => Some(find_dict_path(py, dt)?),
        };

        let mut config = Config::new(config_path, resource_dir, dict_path).map_err(|e| {
            PyException::new_err(format!("Error loading config: {}", e.to_string()))
        })?;

        // sudachi.json does not have systemDict key or its value is ""
        if config.system_dict.is_none() || config.system_dict.as_ref().unwrap().is_dir() {
            config.system_dict = Some(find_dict_path(py, "core")?);
        }

        let dictionary = Arc::new(JapaneseDictionary::from_cfg(&config).map_err(|e| {
            PyException::new_err(format!(
                "Error while constructing dictionary: {}",
                e.to_string()
            ))
        })?);

        Ok(Self {
            dictionary: Some(dictionary),
        })
    }

    /// Creates a sudachi tokenizer
    #[pyo3(text_signature = "($self, mode)")]
    #[args(mode = "None")]
    fn create(&self, mode: Option<PySplitMode>) -> PyTokenizer {
        let tokenizer = StatelessTokenizer::new(self.dictionary.as_ref().unwrap().clone());
        let mode = mode.unwrap_or(PySplitMode::C).into();

        PyTokenizer::new(tokenizer, mode)
    }

    /// Close this dictionary
    #[pyo3(text_signature = "($self)")]
    fn close(&mut self) {
        self.dictionary = None;
    }
}

fn get_default_setting_path(py: Python) -> PyResult<PathBuf> {
    let path = PyModule::import(py, "sudachi")?.getattr("_DEFAULT_SETTINGFILE")?;
    let path = path.cast_as::<PyString>()?.to_str()?;
    Ok(PathBuf::from(path))
}

fn get_default_resource_dir(py: Python) -> PyResult<PathBuf> {
    let path = PyModule::import(py, "sudachi")?.getattr("_DEFAULT_RESOURCEDIR")?;
    let path = path.cast_as::<PyString>()?.to_str()?;
    Ok(PathBuf::from(path))
}

fn find_dict_path(py: Python, dict_type: &str) -> PyResult<PathBuf> {
    let pyfunc = PyModule::import(py, "sudachi")?.getattr("_find_dict_path")?;
    let path = pyfunc
        .call1((dict_type,))?
        .cast_as::<PyString>()?
        .to_str()?;
    Ok(PathBuf::from(path))
}
