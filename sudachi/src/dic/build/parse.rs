/*
 *  Copyright (c) 2021-2026 Works Applications Co., Ltd.
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

use std::borrow::Cow;
use std::str::FromStr;

use lazy_static::lazy_static;
use regex::Regex;

use crate::analysis::Mode;
use crate::dic::build::error::{BuildFailure, DicWriteResult};
use crate::dic::build::{MAX_ARRAY_LEN, MAX_DIC_STRING_LEN};
use crate::dic::word_id::WordRef;

#[inline(always)]
pub fn it_next<'a, I, T, F>(
    orig: &'a str,
    data: &mut I,
    field: &'static str,
    f: F,
) -> DicWriteResult<T>
where
    I: Iterator<Item = &'a str>,
    F: FnOnce(&'a str) -> DicWriteResult<T>,
    T: 'a,
{
    match data.next() {
        Some(s) => f(s),
        None => Err(BuildFailure::SplitFormatError {
            original: orig.to_owned(),
            field,
        }),
    }
}

pub(crate) fn none_if_equal(expected: &str, data: Cow<str>) -> Option<String> {
    if expected == data {
        None
    } else {
        match data {
            Cow::Borrowed(x) => Some(x.to_owned()),
            Cow::Owned(x) => Some(x),
        }
    }
}

#[inline]
pub(crate) fn parse_mode(data: &str) -> DicWriteResult<Mode> {
    match data.trim() {
        "" => Ok(Mode::C),
        "a" | "A" => Ok(Mode::A),
        "b" | "B" => Ok(Mode::B),
        "c" | "C" | "*" => Ok(Mode::C),
        "BC" => Ok(Mode::B),
        _ => Err(BuildFailure::InvalidSplit(data.to_owned())),
    }
}

#[inline]
pub(crate) fn parse_i16(data: &str) -> DicWriteResult<i16> {
    match i16::from_str(data) {
        Ok(v) => Ok(v),
        Err(_) => Err(BuildFailure::InvalidI16Literal(data.to_owned())),
    }
}

#[inline]
pub(crate) fn parse_u32(data: &str) -> DicWriteResult<u32> {
    match u32::from_str(data) {
        Ok(v) => Ok(v),
        Err(_) => Err(BuildFailure::InvalidU32Literal(data.to_owned())),
    }
}

#[inline]
pub(crate) fn parse_v0_line_ref(data: &str) -> DicWriteResult<WordRef> {
    if let Some(stripped) = data.strip_prefix('U') {
        let wref = parse_v0_line_ref_raw(stripped);
        wref.map(|w| WordRef::new(false, w.entry().as_raw()))
    } else {
        parse_v0_line_ref_raw(data)
    }
}

#[inline]
fn parse_v0_line_ref_raw(data: &str) -> DicWriteResult<WordRef> {
    match u32::from_str(data) {
        Ok(v) => match WordRef::checked(true, v) {
            Ok(wref) => Ok(wref),
            Err(_) => Err(BuildFailure::InvalidWordId(data.to_owned())),
        },
        Err(_) => Err(BuildFailure::InvalidWordId(data.to_owned())),
    }
}

#[inline]
pub(crate) fn parse_u32_list_with_asterisk(
    data: &str,
    allow_asterisk: bool,
) -> DicWriteResult<Vec<u32>> {
    if data.is_empty() || data == "*" {
        if data == "*" && !allow_asterisk {
            return Err(BuildFailure::InvalidSplit(data.to_owned()));
        }
        return Ok(Vec::new());
    }

    parse_slash_list(data, parse_u32)
}

lazy_static! {
    // pattern for an entry reference by line number
    pub(crate) static ref LINE_REF_LITERAL: Regex = Regex::new(r"^U?\d+$").unwrap();
}

#[inline]
pub(crate) fn parse_slash_list<T, F>(data: &str, mut f: F) -> DicWriteResult<Vec<T>>
where
    F: FnMut(&str) -> DicWriteResult<T>,
{
    let mut result = Vec::with_capacity(4);

    for part in data.split('/') {
        result.push(f(part)?);
    }

    if result.len() > MAX_ARRAY_LEN {
        return Err(BuildFailure::InvalidSize {
            expected: MAX_ARRAY_LEN,
            actual: result.len(),
        });
    }

    Ok(result)
}

lazy_static! {
    static ref UNICODE_LITERAL: Regex =
        Regex::new(r"\\u(?:\{([0-9a-fA-F]{1,6})\}|([0-9a-fA-F]{4}))").unwrap();
}

fn check_str_len(data: &str) -> DicWriteResult<()> {
    let char_len = data.encode_utf16().count();
    if char_len > MAX_DIC_STRING_LEN {
        Err(BuildFailure::InvalidSize {
            expected: MAX_DIC_STRING_LEN,
            actual: char_len,
        })
    } else {
        Ok(())
    }
}

#[inline]
pub(crate) fn unescape_cow(data: &str) -> DicWriteResult<Cow<'_, str>> {
    check_str_len(data)?;
    if !UNICODE_LITERAL.is_match(data) {
        Ok(Cow::Borrowed(data))
    } else {
        unescape_slow(data).map(Cow::Owned)
    }
}

#[inline]
pub(crate) fn unescape(data: &str) -> DicWriteResult<String> {
    check_str_len(data)?;
    if !UNICODE_LITERAL.is_match(data) {
        Ok(data.to_owned())
    } else {
        unescape_slow(data)
    }
}

#[inline(never)]
fn unescape_slow(original: &str) -> DicWriteResult<String> {
    let mut result = String::with_capacity(original.len());
    let mut start = 0;
    for c in UNICODE_LITERAL.captures_iter(original) {
        let whole = c.get(0).unwrap();
        let braces = c.get(1).or_else(|| c.get(2)).unwrap();
        result.push_str(&original[start..whole.start()]);
        match u32::from_str_radix(braces.as_str(), 16) {
            Ok(c) => match char::from_u32(c) {
                Some(cx) => result.push(cx),
                None => return Err(BuildFailure::InvalidCharLiteral(braces.as_str().to_owned())),
            },
            Err(_) => return Err(BuildFailure::InvalidCharLiteral(braces.as_str().to_owned())),
        }
        start = whole.end();
    }
    result.push_str(&original[start..]);
    Ok(result)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn decode_plain() {
        assert_eq!(unescape("").unwrap(), "");
        assert_eq!(unescape("a").unwrap(), "a");
        assert_eq!(unescape("あ").unwrap(), "あ");
    }

    #[test]
    fn decode_escape_1() {
        assert_eq!(unescape("\\u0020").unwrap(), "\u{20}");
        assert_eq!(unescape("\\u{20}").unwrap(), "\u{20}");
        assert_eq!(unescape("\\u{1f49e}").unwrap(), "💞");
    }

    #[test]
    fn decode_escape_2() {
        assert_eq!(unescape("\\u020f").unwrap(), "\u{20f}");
        assert_eq!(unescape("\\u{20}f").unwrap(), "\u{20}f");
    }

    #[test]
    fn decode_escape_3() {
        assert_eq!(unescape("f\\u0020").unwrap(), "f\u{20}");
        assert_eq!(unescape("f\\u{20}").unwrap(), "f\u{20}");
    }

    #[test]
    fn decode_escape_4() {
        assert_eq!(unescape("\\u100056").unwrap(), "\u{1000}56");
    }

    #[test]
    fn decode_escape_ported() {
        assert_eq!(unescape("a\\u002cc").unwrap(), "a,c");
        assert_eq!(unescape("a\\u{2c}c").unwrap(), "a,c");
    }

    #[test]
    fn decode_escape_fail() {
        assert_eq!(unescape("\\u{10FFFF}").unwrap(), "\u{10FFFF}");
        // max character
        claim::assert_matches!(unescape("\\u{110000}"), Err(_));
        claim::assert_matches!(unescape("\\u{FFFFFF}"), Err(_));
    }
}
