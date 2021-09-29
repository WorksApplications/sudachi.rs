use pyo3::prelude::*;

pub mod dictionary;
pub mod morpheme;
pub mod tokenizer;

/// module root
#[pymodule]
fn sudachi(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<dictionary::PyDictionary>()?;
    m.add_class::<tokenizer::PySplitMode>()?;
    m.add_class::<tokenizer::PyTokenizer>()?;
    m.add_class::<morpheme::PyMorpheme>()?;
    m.add_class::<morpheme::PyWordInfo>()?;
    Ok(())
}
