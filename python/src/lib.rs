use pyo3::prelude::*;

/// module root
#[pymodule]
fn sudachi(_py: Python, _m: &PyModule) -> PyResult<()> {
    Ok(())
}
