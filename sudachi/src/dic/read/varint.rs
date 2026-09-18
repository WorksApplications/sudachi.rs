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

use nom::number::complete::le_u8;

use super::error::{SudachiNomError, SudachiNomResult};

pub fn varint64(input: &[u8]) -> SudachiNomResult<&[u8], u64> {
    let (rest, b0) = le_u8(input)?;
    let b0 = (b0 as u64) & 0xff;
    if b0 < 0x80 {
        return Ok((rest, b0));
    }
    let v0 = b0 & 0x7f;

    varint64_slowpath(v0, rest)
}

fn varint64_slowpath(v0: u64, rest: &[u8]) -> SudachiNomResult<&[u8], u64> {
    let (rest, b1) = le_u8(rest)?;
    let b1 = (b1 as u64) & 0xff;
    if b1 < 0x80 {
        return Ok((rest, (b1 << 7) | v0));
    }
    let v1 = ((b1 & 0x7f) << 7) | v0;

    let (rest, b2) = le_u8(rest)?;
    let b2 = (b2 as u64) & 0xff;
    if b2 < 0x80 {
        return Ok((rest, (b2 << 14) | v1));
    }
    let v2 = ((b2 & 0x7f) << 14) | v1;

    let (rest, b3) = le_u8(rest)?;
    let b3 = (b3 as u64) & 0xff;
    if b3 < 0x80 {
        return Ok((rest, (b3 << 21) | v2));
    }
    let v3 = ((b3 & 0x7f) << 21) | v2;

    let (rest, b4) = le_u8(rest)?;
    let b4 = (b4 as u64) & 0xff;
    if b4 < 0x80 {
        return Ok((rest, (b4 << 28) | v3));
    }
    let v4 = ((b4 & 0x7f) << 28) | v3;

    let (rest, b5) = le_u8(rest)?;
    let b5 = (b5 as u64) & 0xff;
    if b5 < 0x80 {
        return Ok((rest, (b5 << 35) | v4));
    }
    let v5 = ((b5 & 0x7f) << 35) | v4;

    let (rest, b6) = le_u8(rest)?;
    let b6 = (b6 as u64) & 0xff;
    if b6 < 0x80 {
        return Ok((rest, (b6 << 42) | v5));
    }
    let v6 = ((b6 & 0x7f) << 42) | v5;

    let (rest, b7) = le_u8(rest)?;
    let b7 = (b7 as u64) & 0xff;
    if b7 < 0x80 {
        return Ok((rest, (b7 << 49) | v6));
    }
    let v7 = ((b7 & 0x7f) << 49) | v6;

    let (rest, b8) = le_u8(rest)?;
    let b8 = (b8 as u64) & 0xff;
    if b8 < 0x80 {
        return Ok((rest, (b8 << 56) | v7));
    }
    let v8 = ((b8 & 0x7f) << 56) | v7;

    let (rest, b9) = le_u8(rest)?;
    let b9 = (b9 as u64) & 0xff;
    if b9 < 0x02 {
        // only 1 bits are valid here, rest must be 0
        Ok((rest, (b9 << 63) | v8))
    } else {
        Err(nom::Err::Failure(SudachiNomError::InvalidVarInt))
    }
}

pub fn varint32(input: &[u8]) -> SudachiNomResult<&[u8], u32> {
    let (rest, v) = varint64(input)?;
    if (v & !0xffffffff) != 0 {
        Err(nom::Err::Failure(SudachiNomError::InvalidVarInt))
    } else {
        Ok((rest, v as u32))
    }
}
