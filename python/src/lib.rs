use std::path::PathBuf;
use std::sync::Arc;

use pyo3::exceptions::PyException;
use pyo3::prelude::*;

use sudachi::config::Config;
use sudachi::dic::dictionary::JapaneseDictionary;
use sudachi::prelude::*;
use sudachi::stateless_tokeniser::StatelessTokenizer;

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
        let config = Config::new(config_path, resource_dir, None)
            .map_err(|e| PyException::new_err(format!("Error loding config: {}", e.to_string())))?;

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

        let tokenizer = StatelessTokenizer::new(self.dictionary.clone());

        Ok(PyTokenizer { tokenizer, mode })
    }
}

#[pyclass]
pub struct PyTokenizer {
    tokenizer: StatelessTokenizer<Arc<JapaneseDictionary>>,
    mode: Mode,
}

#[pymethods]
impl PyTokenizer {
    fn tokenize(&self, input: &str, enable_debug: Option<bool>) -> PyResult<Vec<String>> {
        Ok(self
            .tokenizer
            .tokenize(input, self.mode, enable_debug.unwrap_or(false))
            .map_err(|e| {
                PyException::new_err(format!("Error while tokenization: {}", e.to_string()))
            })?
            .iter()
            .map(|m| m.surface().clone())
            .collect())
    }
}
