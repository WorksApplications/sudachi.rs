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

use pyo3::prelude::*;
use pyo3::wrap_pymodule;

mod dictionary;
mod morpheme;
mod tokenizer;
mod word_info;

#[pymodule]
fn dictionary(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<dictionary::PyDictionary>()?;
    Ok(())
}

#[pymodule]
fn tokenizer(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<tokenizer::PySplitMode>()?;
    m.add_class::<tokenizer::PyTokenizer>()?;
    Ok(())
}

#[pymodule]
fn morpheme(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<morpheme::PyMorpheme>()?;
    Ok(())
}

#[pymodule]
fn morphemelist(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<morpheme::PyMorphemeListWrapper>()?;
    Ok(())
}

/// module root
#[pymodule]
fn sudachi(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_wrapped(wrap_pymodule!(dictionary))?;
    m.add_wrapped(wrap_pymodule!(tokenizer))?;
    m.add_wrapped(wrap_pymodule!(morpheme))?;
    m.add_wrapped(wrap_pymodule!(morphemelist))?;

    m.add_class::<dictionary::PyDictionary>()?;
    m.add_class::<tokenizer::PySplitMode>()?;
    m.add_class::<tokenizer::PyTokenizer>()?;
    m.add_class::<morpheme::PyMorphemeListWrapper>()?;
    m.add_class::<morpheme::PyMorpheme>()?;
    m.add_class::<word_info::PyWordInfo>()?;
    Ok(())
}
