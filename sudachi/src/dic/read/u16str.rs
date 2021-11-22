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

use crate::error::{SudachiNomError, SudachiNomResult};
use nom::number::complete::le_u8;
use std::iter::FusedIterator;

pub fn utf16_string_parser(input: &[u8]) -> SudachiNomResult<&[u8], String> {
    utf16_string_data(input).and_then(|(rest, data)| {
        if data.is_empty() {
            Ok((rest, String::new()))
        } else {
            // most Japanese chars are 3-bytes in utf-8 and 2 in utf-16
            let capacity = (data.len() + 1) * 3 / 2;
            let mut result = String::with_capacity(capacity);
            let iter = U16CodeUnits::new(data);
            for c in char::decode_utf16(iter) {
                match c {
                    Err(_) => return Err(nom::Err::Failure(SudachiNomError::Utf16String)),
                    Ok(c) => result.push(c),
                }
            }
            Ok((rest, result))
        }
    })
}

pub fn skip_u16_string(input: &[u8]) -> SudachiNomResult<&[u8], String> {
    utf16_string_data(input).map(|(rest, _)| (rest, String::new()))
}

#[inline]
pub fn utf16_string_data(input: &[u8]) -> SudachiNomResult<&[u8], &[u8]> {
    let (rest, length) = string_length_parser(input)?;
    if length == 0 {
        return Ok((rest, &[]));
    }
    let num_bytes = (length * 2) as usize;
    if rest.len() < num_bytes {
        return Err(nom::Err::Failure(SudachiNomError::Utf16String));
    }

    let (data, rest) = rest.split_at(num_bytes);

    Ok((rest, data))
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

/// Read UTF-16 code units from non-aligned storage
pub struct U16CodeUnits<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> U16CodeUnits<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        U16CodeUnits { data, offset: 0 }
    }
}

impl Iterator for U16CodeUnits<'_> {
    type Item = u16;

    fn next(&mut self) -> Option<Self::Item> {
        if self.data.len() <= self.offset {
            return None;
        }
        let p1 = self.data[self.offset];
        let p2 = self.data[self.offset + 1];
        self.offset += 2;
        Some(u16::from_le_bytes([p1, p2]))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let rem = self.data.len() - self.offset;
        (rem, Some(rem))
    }
}

impl FusedIterator for U16CodeUnits<'_> {}
