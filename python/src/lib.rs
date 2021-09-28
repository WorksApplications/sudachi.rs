use pyo3::prelude::*;

pub mod dictionary;
pub mod morpheme;
pub mod tokenizer;

use crate::dictionary::PyDictionary;
use crate::morpheme::PyMorpheme;
use crate::tokenizer::{PySplitMode, PyTokenizer};

/// module root
#[pymodule]
fn sudachi(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PySplitMode>()?;
    m.add_class::<PyDictionary>()?;
    m.add_class::<PyTokenizer>()?;
    m.add_class::<PyMorpheme>()?;
    Ok(())
}
