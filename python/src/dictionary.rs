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
use std::str::FromStr;
use std::sync::Arc;

use pyo3::exceptions::{self, PyException};
use pyo3::prelude::*;

use sudachi::analysis::stateless_tokenizer::StatelessTokenizer;
use sudachi::config::Config;
use sudachi::dic::dictionary::JapaneseDictionary;

use crate::tokenizer::{PySplitMode, PyTokenizer};

pub enum DictionaryType {
    Small,
    Core,
    Full,
}

impl DictionaryType {
    fn to_str(&self) -> &str {
        match self {
            Self::Small => "small",
            Self::Core => "core",
            Self::Full => "full",
        }
    }
}

impl FromStr for DictionaryType {
    type Err = PyErr;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "small" => Ok(Self::Small),
            "core" => Ok(Self::Core),
            "full" => Ok(Self::Full),
            _ => Err(PyErr::new::<exceptions::PyValueError, _>(
                "dict_type must be \"small\", \"core\", or \"full\"",
            )),
        }
    }
}

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
        let dict_path = match dict_type {
            None => None,
            Some(dt) => {
                // let dict_type = DictionaryType::from_str(dt)?;
                // Some(find_dict_path(py, dict_type)?)
                Some(find_dict_path_py(py, dt)?)
            }
        };

        let mut config = Config::new(config_path, resource_dir, dict_path).map_err(|e| {
            PyException::new_err(format!("Error loading config: {}", e.to_string()))
        })?;

        // sudachi.json does not have systemDict key or its value is ""
        if config.system_dict.is_none() || config.system_dict.as_ref().unwrap().is_dir() {
            // config.system_dict = Some(find_dict_path(py, DictionaryType::Core)?);
            config.system_dict = Some(find_dict_path_py(py, "core")?);
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

fn find_dict_path(py: Python, dict_type: DictionaryType) -> PyResult<PathBuf> {
    let pkg_name = String::from("sudachidict_") + dict_type.to_str();
    let module_spec = PyModule::import(py, "importlib.util")?
        .getattr("find_spec")?
        .call1((&pkg_name,))?;

    if module_spec.is_none() {
        return Err(PyErr::new::<exceptions::PyModuleNotFoundError, _>(format!(
            "Package `{}` does not exist.\nYou may install it with a command `$ pip install {}`",
            &pkg_name, &pkg_name
        )));
    }

    get_absolute_dict_path(py, dict_type)
}

fn get_absolute_dict_path(py: Python, dict_type: DictionaryType) -> PyResult<PathBuf> {
    let pkg_name = String::from("sudachidict_") + dict_type.to_str();
    let pkg_path = PyModule::import(py, &pkg_name)?
        .getattr("__file__")?
        .cast_as::<pyo3::types::PyString>()?
        .to_str()?;
    let dict_path = PathBuf::from(pkg_path)
        .parent()
        .unwrap()
        .join("resources/system.dic");

    Ok(dict_path)
}

fn find_dict_path_py(py: Python, dict_type: &str) -> PyResult<PathBuf> {
    let source_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("py_src")
        .join("sudachi")
        .join("dictionary_path.py");
    let code = std::fs::read_to_string(source_file)?;
    let module = PyModule::from_code(py, &code, "file", "module")?;
    let path = PathBuf::from(
        module
            .getattr("find_dict_path")?
            .call1((dict_type,))?
            .cast_as::<pyo3::types::PyString>()?
            .to_str()?,
    );

    Ok(path)
}
