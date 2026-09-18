/*
 *  Copyright (c) 2025 Works Applications Co., Ltd.
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

pub type SudachiNomResult<I, O> = nom::IResult<I, O, SudachiNomError<I>>;

/// Custum nom error
#[derive(Debug, PartialEq)]
pub enum SudachiNomError<I> {
    /// Failed to parse utf16 string
    Utf16String,
    /// Failed to parse utf8 string
    Utf8String,
    /// Failed to parse variable-length integer
    InvalidVarInt,
    Nom(I, nom::error::ErrorKind),
    OutOfBounds(String, usize, usize),
}

impl<I> nom::error::ParseError<I> for SudachiNomError<I> {
    fn from_error_kind(input: I, kind: nom::error::ErrorKind) -> Self {
        SudachiNomError::Nom(input, kind)
    }
    fn append(_: I, _: nom::error::ErrorKind, other: Self) -> Self {
        other
    }
}
