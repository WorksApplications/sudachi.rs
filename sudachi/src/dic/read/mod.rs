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

mod u16str;

use nom::number::complete::{le_u16, le_u32, le_u8};
use nom::Parser;

use crate::dic::word_id::WordId;
use crate::error::{SudachiNomError, SudachiNomResult};

pub fn u32_array_parser(input: &[u8]) -> SudachiNomResult<&[u8], Vec<u32>> {
    let (rest, length) = le_u8(input)?;
    nom::multi::count(le_u32, length as usize)(rest)
}

pub fn u32_wid_array_parser(input: &[u8]) -> SudachiNomResult<&[u8], Vec<WordId>> {
    let (rest, length) = le_u8(input)?;
    nom::multi::count(le_u32.map(|id| WordId::from_raw(id)), length as usize)(rest)
}

pub fn utf16_string_parser(input: &[u8]) -> SudachiNomResult<&[u8], String> {
    let (rest, length) = string_length_parser(input)?;
    if length == 0 {
        return Ok((rest, String::new()));
    }
    let num_bytes = (length * 2) as usize;
    if rest.len() < num_bytes {
        return Err(nom::Err::Failure(SudachiNomError::Utf16String));
    }

    let mut result = String::with_capacity(num_bytes * 2);
    let iter = u16str::U16CodeUnits::new(&rest[..num_bytes]);
    for c in char::decode_utf16(iter) {
        match c {
            Err(_) => return Err(nom::Err::Failure(SudachiNomError::Utf16String)),
            Ok(c) => result.push(c),
        }
    }
    Ok((&rest[num_bytes..], result))
}

pub fn string_length_parser(input: &[u8]) -> SudachiNomResult<&[u8], u16> {
    let (rest, length) = le_u8(input)?;
    // word length can be 1 or 2 bytes
    let (rest, opt_low) = nom::combinator::cond(length >= 128, le_u8)(rest)?;
    Ok((
        rest,
        match opt_low {
            Some(low) => ((length as u16 & 0x7F) << 8) | low as u16,
            None => length as u16,
        },
    ))
}
