use std::path::PathBuf;
use std::sync::Arc;

use pyo3::exceptions::PyException;
use pyo3::prelude::*;

use sudachi::config::Config;
use sudachi::dic::dictionary::JapaneseDictionary;
use sudachi::prelude::*;
use sudachi::stateless_tokeniser::StatelessTokenizer;

pub mod morpheme;
use crate::morpheme::PyMorpheme;

/// module root
#[pymodule]
fn sudachi(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyDictionary>()?;
    m.add_class::<PyTokenizer>()?;
    Ok(())
}

#[pyclass]
pub struct PyDictionary {
    dictionary: Arc<JapaneseDictionary>,
}

#[pymethods]
impl PyDictionary {
    /// Creates a sudachi dictionary
    #[new]
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
    fn create(&self, mode: Option<&str>) -> PyResult<PyTokenizer> {
        let mode: Mode = mode
            .unwrap_or("C")
            .parse()
            .map_err(|e: &str| PyException::new_err(format!("Error: {}", e)))?;

        let dictionary = self.dictionary.clone();
        let tokenizer = StatelessTokenizer::new(self.dictionary.clone());

        Ok(PyTokenizer {
            dictionary,
            tokenizer,
            mode,
        })
    }
}

#[pyclass]
pub struct PyTokenizer {
    dictionary: Arc<JapaneseDictionary>,
    tokenizer: StatelessTokenizer<Arc<JapaneseDictionary>>,
    mode: Mode,
}

#[pymethods]
impl PyTokenizer {
    // want to take logger instead of deug flag
    fn tokenize(
        &self,
        text: &str,
        mode: Option<&str>,
        enable_debug: Option<bool>,
    ) -> PyResult<Vec<PyMorpheme>> {
        let mode: Mode = match mode {
            Some(m) => m
                .parse()
                .map_err(|e: &str| PyException::new_err(format!("Error: {}", e)))?,
            None => self.mode,
        };

        let morphemes = self
            .tokenizer
            .tokenize(text, mode, enable_debug.unwrap_or(false))
            .map_err(|e| {
                PyException::new_err(format!("Error while tokenization: {}", e.to_string()))
            })?
            .into_iter()
            .map(|m| PyMorpheme::new(m, self.dictionary.clone()))
            .collect();

        Ok(morphemes)
    }
}
