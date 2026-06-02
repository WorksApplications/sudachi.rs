/*
 *  Copyright (c) 2021-2024 Works Applications Co., Ltd.
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

use std::fs::{File, OpenOptions};
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyList, PyString, PyType};

use sudachi::analysis::stateless_tokenizer::DictionaryAccess;
use sudachi::config::Config;
use sudachi::dic::build::{AsDataSource, DataSource, DictBuilder};
use sudachi::dic::dictionary::JapaneseDictionary;

use crate::dictionary::get_default_resource_dir;
use crate::errors;

pub fn register_functions(m: &Bound<PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(build_system_dic, m)?)?;
    m.add_function(wrap_pyfunction!(build_user_dic, m)?)?;
    Ok(())
}

enum PyDataSource<'py> {
    File(PathBuf),
    Data(&'py [u8]),
}

impl<'a, 'py: 'a> AsDataSource<'a> for &'a PyDataSource<'py> {
    fn convert(self) -> DataSource<'a> {
        match self {
            PyDataSource::File(path) => DataSource::File(path),
            PyDataSource::Data(data) => DataSource::Data(data),
        }
    }

    fn name(&self) -> String {
        match self {
            PyDataSource::File(path) => path.to_str().map(|s| s.to_owned()).unwrap_or_default(),
            PyDataSource::Data(data) => format!("memory ({} bytes)", data.len()),
        }
    }
}

fn to_stats<T: DictionaryAccess>(py: Python, builder: DictBuilder<T>) -> PyResult<Bound<PyList>> {
    let stats = PyList::empty(py);

    for p in builder.report() {
        let values = (p.part(), p.size(), p.time().as_secs_f64());
        stats.append(values.into_pyobject(py)?)?;
    }

    Ok(stats)
}

fn create_file(p: &Path) -> std::io::Result<File> {
    if p.exists() {
        std::fs::remove_file(p)?;
    }

    OpenOptions::new().create_new(true).write(true).open(p)
}

/// Build system dictionary from matrix and lexicons.
///
/// :param matrix: Path to the matrix file.
/// :param lex: List of paths to lexicon files.
/// :param output: Path to output built dictionary.
/// :param description: A description text to embed in the dictionary.
/// :return: A build report, list of (part, size, time).
///
/// :type matrix: pathlib.Path | str | bytes
/// :type lex: list[pathlib.Path | str | bytes]
/// :type output: pathlib.Path | str
/// :type description: str
#[pyfunction]
#[pyo3(
    signature = (matrix, lex, output, description=None),
    text_signature = "(matrix, lex, output, description=None) -> list[tuple[str, int, float]]",
)]
fn build_system_dic<'py>(
    py: Python<'py>,
    matrix: &Bound<'py, PyAny>,
    lex: &Bound<'py, PyList>,
    output: &Bound<'py, PyAny>,
    description: Option<&str>,
) -> PyResult<Bound<'py, PyList>> {
    let mut builder = DictBuilder::new_system();
    if let Some(d) = description {
        builder.set_description(d)
    }

    let matrix_path = resolve_as_pypathstr(py, matrix)?;
    let matrix_src = as_data_source(matrix_path.as_ref(), matrix)?;
    errors::wrap_ctx(builder.read_conn(&matrix_src), matrix)?;
    for f in lex.iter() {
        let lex_path = resolve_as_pypathstr(py, &f)?;
        let lex_src = as_data_source(lex_path.as_ref(), &f)?;
        errors::wrap_ctx(builder.read_lexicon(&lex_src), &f)?;
    }
    let out_path = resolve_as_pypathstr(py, output)?;
    let out_src = as_data_source(out_path.as_ref(), output)?;
    let out_file = match &out_src {
        PyDataSource::File(p) => errors::wrap_ctx(create_file(p), p)?,
        PyDataSource::Data(_) => return errors::wrap(Err("can't use bytes for output")),
    };
    let mut buf_writer = BufWriter::new(out_file);
    errors::wrap(builder.resolve())?;
    errors::wrap(builder.compile(&mut buf_writer))?;

    to_stats(py, builder)
}

/// Build user dictionary from lexicons based on the given system dictionary.
///
/// :param system: Path to the system dictionary.
/// :param lex: List of paths to lexicon files.
/// :param output: Path to output built dictionary.
/// :param description: A description text to embed in the dictionary.
/// :return: A build report, list of (part, size, time).
///
/// :type system: pathlib.Path | str
/// :type lex: list[pathlib.Path | str | bytes]
/// :type output: pathlib.Path | str
/// :type description: str
#[pyfunction]
#[pyo3(
    signature = (system, lex, output, description=None),
    text_signature = "(system, lex, output, description=None) -> list[tuple[str, int, float]]",
)]
fn build_user_dic<'py>(
    py: Python<'py>,
    system: &Bound<'py, PyAny>,
    lex: &Bound<'py, PyList>,
    output: &Bound<'py, PyAny>,
    description: Option<&str>,
) -> PyResult<Bound<'py, PyList>> {
    let system_path = resolve_as_pypathstr(py, system)?;
    let system_src = as_data_source(system_path.as_ref(), system)?;
    let system_dic = match &system_src {
        PyDataSource::File(f) => {
            let resource_path = get_default_resource_dir(py)?;
            let cfg = Config::minimal_at(resource_path).with_system_dic(f);
            errors::wrap_ctx(JapaneseDictionary::from_cfg(&cfg), f)?
        }
        PyDataSource::Data(_) => {
            return errors::wrap(Err(
                "can't load system dictionary from bytes, pass path to the file",
            ))
        }
    };

    let mut builder = DictBuilder::new_user(&system_dic);
    if let Some(d) = description {
        builder.set_description(d)
    }

    for f in lex.iter() {
        let lex_path = resolve_as_pypathstr(py, &f)?;
        let lex_src = as_data_source(lex_path.as_ref(), &f)?;
        errors::wrap_ctx(builder.read_lexicon(&lex_src), &f)?;
    }
    let out_path = resolve_as_pypathstr(py, output)?;
    let out_src = as_data_source(out_path.as_ref(), output)?;
    let out_file = match &out_src {
        PyDataSource::File(p) => errors::wrap_ctx(create_file(p), p)?,
        PyDataSource::Data(_) => return errors::wrap(Err("can't use bytes for output")),
    };
    let mut buf_writer = BufWriter::new(out_file);
    errors::wrap(builder.resolve())?;
    errors::wrap(builder.compile(&mut buf_writer))?;

    to_stats(py, builder)
}

fn resolve_as_pypathstr<'py>(
    py: Python<'py>,
    data: &Bound<'py, PyAny>,
) -> PyResult<Option<Bound<'py, PyString>>> {
    let binding = py.import("pathlib")?.getattr("Path")?;
    let path = binding.cast::<PyType>()?;
    if data.is_instance(path)? {
        Ok(Some(data.call_method0("resolve")?.str()?))
    } else if data.is_instance_of::<PyString>() {
        Ok(Some(data.str()?))
    } else {
        Ok(None)
    }
}

fn as_data_source<'py>(
    resolved_path: Option<&'py Bound<'py, PyString>>,
    original_obj: &'py Bound<'py, PyAny>,
) -> PyResult<PyDataSource<'py>> {
    match resolved_path {
        Some(pystr) => Ok(PyDataSource::File(PathBuf::from(pystr.to_cow()?.as_ref()))),
        None => {
            if original_obj.is_instance_of::<PyBytes>() {
                Ok(PyDataSource::Data(
                    original_obj.cast::<PyBytes>()?.as_bytes(),
                ))
            } else {
                errors::wrap(Err(format!(
                    "data source should be only Path, bytes or str, was {}: {}",
                    original_obj,
                    original_obj.get_type()
                )))
            }
        }
    }
}
