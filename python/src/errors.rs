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

use pyo3::exceptions::PyException;
use pyo3::PyResult;
use std::fmt::{Debug, Display};

pub fn wrap<T, E: Display>(v: Result<T, E>) -> PyResult<T> {
    match v {
        Ok(v) => Ok(v),
        Err(e) => Err(PyException::new_err(format!("{}", e))),
    }
}

pub fn wrap_ctx<T, E: Display, C: Debug + ?Sized>(v: Result<T, E>, ctx: &C) -> PyResult<T> {
    match v {
        Ok(v) => Ok(v),
        Err(e) => Err(PyException::new_err(format!("{:?}: {}", ctx, e))),
    }
}
