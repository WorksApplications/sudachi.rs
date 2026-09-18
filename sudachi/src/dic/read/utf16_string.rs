/*
 *  Copyright (c) 2025-2026 Works Applications Co., Ltd.
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

use nom::number::complete::le_i16;

use super::error::{SudachiNomError, SudachiNomResult};

pub fn utf16_string_of_length(input: &[u8], char_length: usize) -> SudachiNomResult<&[u8], String> {
    if char_length == 0 {
        return Ok((input, String::new()));
    }

    let num_bytes = char_length * 2;
    let (data, rest) = input
        .split_at_checked(num_bytes)
        .ok_or(nom::Err::Failure(SudachiNomError::Utf16String))?;

    let decoded = string_from_utf16le(data).map_err(nom::Err::Failure)?;
    Ok((rest, decoded))
}

pub fn utf16_string(input: &[u8]) -> SudachiNomResult<&[u8], String> {
    let (rest, char_length) = le_i16(input)?;
    utf16_string_of_length(rest, char_length as usize)
}

pub fn skip_utf16_string(input: &[u8]) -> SudachiNomResult<&[u8], String> {
    let (rest, length) = le_i16(input)?;
    let num_bytes = (length * 2) as usize;
    let (rest, _) = nom::bytes::complete::take(num_bytes)(rest)?;
    Ok((rest, String::new()))
}

// nightly feature exists: https://github.com/rust-lang/rust/issues/116258
pub fn string_from_utf16le(bytes: &[u8]) -> Result<String, SudachiNomError<&[u8]>> {
    if bytes.len() % 2 != 0 {
        return Err(SudachiNomError::Utf16String);
    }
    match (cfg!(target_endian = "little"), unsafe {
        bytes.align_to::<u16>()
    }) {
        (true, ([], v, [])) => String::from_utf16(v).map_err(|_| SudachiNomError::Utf16String),
        _ => char::decode_utf16(bytes.chunks(2).map(|v| u16::from_le_bytes([v[0], v[1]])))
            .collect::<Result<String, _>>()
            .map_err(|_| SudachiNomError::Utf16String),
    }
}
