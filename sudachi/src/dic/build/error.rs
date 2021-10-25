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

use crate::prelude::SudachiResult;
use std::error::Error;
use thiserror::Error;

#[derive(Error, Debug)]
#[error("{file}:{line}\t{cause}")]
pub struct DicWriteError {
    file: String,
    line: usize,
    cause: DicWriteReason,
}

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum DicWriteReason {
    #[error("The actual size {actual} was larger than expected {expected}")]
    InvalidSize { actual: usize, expected: usize },

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

struct DicWriteContext {
    name: String,
    line: usize,
}

impl DicWriteContext {
    pub fn memory() -> Self {
        DicWriteContext {
            name: "<memory>".to_owned(),
            line: 0,
        }
    }

    pub fn err<T, E: Into<DicWriteReason>>(self, reason: E) -> SudachiResult<T> {
        match reason.into() {
            DicWriteReason::Io(e) => Err(e.into()),
            reason => {
                let err = DicWriteError {
                    file: self.name,
                    line: self.line,
                    cause: reason,
                };
                Err(err.into())
            }
        }
    }
}

pub type DicWriteResult<T> = std::result::Result<T, DicWriteReason>;
