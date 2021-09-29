use std::path::PathBuf;
use std::sync::Arc;

use pyo3::exceptions::PyException;
use pyo3::prelude::*;

use sudachi::config::Config;
use sudachi::dic::dictionary::JapaneseDictionary;
use sudachi::stateless_tokeniser::StatelessTokenizer;

use crate::tokenizer::{PySplitMode, PyTokenizer};

#[pyclass(module = "sudachi.dictionary", name = "Dictionary")]
#[pyo3(text_signature = "(config_path, resource_dir)")]
pub struct PyDictionary {
    dictionary: Arc<JapaneseDictionary>,
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
        let dictionary = self.dictionary.clone();
        let tokenizer = StatelessTokenizer::new(self.dictionary.clone());
        let mode = mode.unwrap_or(PySplitMode::C).into();

        PyTokenizer::new(dictionary, tokenizer, mode)
    }
}
