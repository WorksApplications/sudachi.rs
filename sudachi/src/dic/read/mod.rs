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

pub(crate) mod u16str;
pub(crate) mod word_info;

use nom::number::complete::{le_u32, le_u8};
use nom::Parser;

use crate::dic::word_id::WordId;
use crate::error::SudachiNomResult;

pub fn u32_array_parser(input: &[u8]) -> SudachiNomResult<&[u8], Vec<u32>> {
    let (rest, length) = le_u8(input)?;
    nom::multi::count(le_u32, length as usize)(rest)
}

pub fn u32_wid_array_parser(input: &[u8]) -> SudachiNomResult<&[u8], Vec<WordId>> {
    let (rest, length) = le_u8(input)?;
    nom::multi::count(le_u32.map(|id| WordId::from_raw(id)), length as usize)(rest)
}

pub fn skip_wid_array(input: &[u8]) -> SudachiNomResult<&[u8], Vec<WordId>> {
    let (rest, length) = le_u8(input)?;
    let num_bytes = length as usize * 4;
    let next = &rest[num_bytes..];
    Ok((next, Vec::new()))
}

pub fn skip_u32_array(input: &[u8]) -> SudachiNomResult<&[u8], Vec<u32>> {
    let (rest, length) = le_u8(input)?;
    let num_bytes = length as usize * 4;
    let next = &rest[num_bytes..];
    Ok((next, Vec::new()))
}

pub fn u32_parser(input: &[u8]) -> SudachiNomResult<&[u8], u32> {
    le_u32(input)
}
