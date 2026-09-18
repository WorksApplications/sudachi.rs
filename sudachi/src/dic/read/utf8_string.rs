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

use super::error::{SudachiNomError, SudachiNomResult};
use super::varint::varint32;

pub fn utf8_string(input: &[u8]) -> SudachiNomResult<&[u8], String> {
    let (rest, length) = varint32(input)?;

    let (data, rest) = rest
        .split_at_checked(length as usize)
        .ok_or(nom::Err::Failure(SudachiNomError::Utf8String))?;

    match std::str::from_utf8(data) {
        Ok(s) => Ok((rest, s.to_string())),
        Err(_) => Err(nom::Err::Failure(SudachiNomError::Utf8String)),
    }
}
