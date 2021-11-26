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

use crate::dictionary::get_default_resource_dir;
use crate::errors;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyList, PyString, PyTuple, PyType};
use std::fs::{File, OpenOptions};
use std::io::BufWriter;
use std::path::Path;
use sudachi::analysis::stateless_tokenizer::DictionaryAccess;
use sudachi::config::Config;
use sudachi::dic::build::{DataSource, DictBuilder};
use sudachi::dic::dictionary::JapaneseDictionary;

pub fn register_functions(m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(build_system_dic, m)?)?;
    m.add_function(wrap_pyfunction!(build_user_dic, m)?)?;
    Ok(())
}

fn to_stats<T: DictionaryAccess>(py: Python, builder: DictBuilder<T>) -> PyResult<&PyList> {
    let stats = PyList::empty(py);

    for p in builder.report() {
        let t = PyTuple::new(
            py,
            [
                p.part().into_py(py),
                p.size().into_py(py),
                p.time().as_secs_f64().into_py(py),
            ],
        );
        stats.append(t)?;
    }

    Ok(stats)
}

fn create_file(p: &Path) -> std::io::Result<File> {
    if p.exists() {
        std::fs::remove_file(p)?;
    }

    OpenOptions::new().create_new(true).write(true).open(p)
}

#[pyfunction]
#[pyo3(text_signature = "(matrix, lex, output, description=None) -> list")]
fn build_system_dic<'p>(
    py: Python<'p>,
    matrix: &'p PyAny,
    lex: &'p PyList,
    output: &'p PyAny,
    description: Option<&str>,
) -> PyResult<&'p PyList> {
    let mut builder = DictBuilder::new_system();
    description.map(|d| builder.set_description(d));

    let matrix_src = as_data_source(&py, matrix)?;
    errors::wrap_ctx(builder.read_conn(matrix_src), matrix)?;
    for f in lex.iter() {
        let lex_src = as_data_source(&py, &f)?;
        errors::wrap_ctx(builder.read_lexicon(lex_src), &f)?;
    }
    let out_file = match as_data_source(&py, output)? {
        DataSource::File(p) => errors::wrap_ctx(create_file(p), p)?,
        DataSource::Data(_) => return errors::wrap(Err("can't use bytes for output")),
    };
    let mut buf_writer = BufWriter::new(out_file);
    errors::wrap(builder.resolve())?;
    errors::wrap(builder.compile(&mut buf_writer))?;

    to_stats(py, builder)
}

#[pyfunction]
#[pyo3(text_signature = "(system, lex, output, description=None) -> list")]
fn build_user_dic<'p>(
    py: Python<'p>,
    system: &'p PyAny,
    lex: &'p PyList,
    output: &'p PyAny,
    description: Option<&str>,
) -> PyResult<&'p PyList> {
    let system_dic = match as_data_source(&py, system)? {
        DataSource::File(f) => {
            let resource_path = get_default_resource_dir(py)?;
            let cfg = Config::minimal_at(resource_path).with_system_dic(f);
            errors::wrap_ctx(JapaneseDictionary::from_cfg(&cfg), f)?
        }
        DataSource::Data(_) => {
            return errors::wrap(Err(
                "can't load system dictionary from bytes, pass path to the file",
            ))
        }
    };

    let mut builder = DictBuilder::new_user(&system_dic);
    description.map(|d| builder.set_description(d));

    for f in lex.iter() {
        let lex_src = as_data_source(&py, &f)?;
        errors::wrap_ctx(builder.read_lexicon(lex_src), &f)?;
    }
    let out_file = match as_data_source(&py, output)? {
        DataSource::File(p) => errors::wrap_ctx(create_file(p), p)?,
        DataSource::Data(_) => return errors::wrap(Err("can't use bytes for output")),
    };
    let mut buf_writer = BufWriter::new(out_file);
    errors::wrap(builder.resolve())?;
    errors::wrap(builder.compile(&mut buf_writer))?;

    to_stats(py, builder)
}

fn as_data_source<'p>(py: &'p Python<'p>, data: &'p PyAny) -> PyResult<DataSource<'p>> {
    let path = py.import("pathlib")?.getattr("Path")?.cast_as::<PyType>()?;
    if path.is_instance(data)? {
        let pypath = data.call_method0("resolve")?.str()?;
        Ok(DataSource::File(Path::new(pypath.to_str()?)))
    } else if data.is_instance::<PyString>()? {
        let pypath = data.str()?;
        Ok(DataSource::File(Path::new(pypath.to_str()?)))
    } else if data.is_instance::<PyBytes>()? {
        let data = data.cast_as::<PyBytes>()?;
        Ok(DataSource::Data(data.as_bytes()))
    } else {
        Err(pyo3::exceptions::PyValueError::new_err(format!(
            "data source should can be only Path, bytes or str, was {}: {}",
            data,
            data.get_type()
        )))
    }
}
