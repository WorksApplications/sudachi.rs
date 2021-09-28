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
    m.add_class::<PySplitMode>()?;
    m.add_class::<PyDictionary>()?;
    m.add_class::<PyTokenizer>()?;
    m.add_class::<PyMorpheme>()?;
    Ok(())
}

/// This implementation is a workaround. Waiting for pyo3 enum feature.
/// ref: [PyO3 issue #834](https://github.com/PyO3/pyo3/issues/834).
#[pyclass]
#[derive(Clone, PartialEq, Eq)]
pub struct PySplitMode {
    mode: u8,
}

#[pymethods]
impl PySplitMode {
    #[classattr]
    const A: Self = Self { mode: 0 };
    #[classattr]
    const B: Self = Self { mode: 1 };
    #[classattr]
    const C: Self = Self { mode: 2 };
}

impl From<Mode> for PySplitMode {
    fn from(mode: Mode) -> Self {
        match mode {
            Mode::A => PySplitMode::A,
            Mode::B => PySplitMode::B,
            Mode::C => PySplitMode::C,
        }
    }
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

impl std::str::FromStr for PySplitMode {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "A" | "a" => Ok(PySplitMode::A),
            "B" | "b" => Ok(PySplitMode::B),
            "C" | "c" => Ok(PySplitMode::C),
            _ => Err("Mode must be one of \"A\", \"B\", or \"C\" (in lower or upper case)."),
        }
    }
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
    fn create(&self, mode: Option<PySplitMode>) -> PyTokenizer {
        let dictionary = self.dictionary.clone();
        let tokenizer = StatelessTokenizer::new(self.dictionary.clone());
        let mode = mode.unwrap_or(PySplitMode::C).into();

        PyTokenizer {
            dictionary,
            tokenizer,
            mode,
        }
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
        mode: Option<PySplitMode>,
        enable_debug: Option<bool>,
    ) -> PyResult<Vec<PyMorpheme>> {
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
            .into_iter()
            .map(|m| PyMorpheme::new(m, self.dictionary.clone()))
            .collect();

        Ok(morphemes)
    }
}
