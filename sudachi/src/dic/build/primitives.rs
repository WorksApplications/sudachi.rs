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

use crate::dic::build::error::BuildFailure::InvalidSize;
use crate::dic::build::error::DicWriteResult;
use crate::dic::build::lexicon::SplitUnit;
use crate::dic::word_id::WordId;
use std::io::Write;

pub struct Utf16Writer {
    buffer: Vec<u8>,
}

impl Utf16Writer {
    pub fn new() -> Self {
        Utf16Writer {
            buffer: Vec::with_capacity(256),
        }
    }

    pub fn write_len<W: Write>(&self, w: &mut W, length: usize) -> DicWriteResult<usize> {
        if length > i16::MAX as _ {
            return Err(InvalidSize {
                actual: length,
                expected: i16::MAX as _,
            });
        }

        let length = length as u16;

        let prefix = if length < 127 {
            w.write_all(&[length as u8])?;
            1
        } else {
            let b0 = (length as u8) & 0xff;
            let b1 = ((length >> 8) as u8) | 0x80;
            w.write_all(&[b1, b0])?;
            2
        };

        Ok(prefix)
    }

    pub fn write<W: Write, T: AsRef<str>>(&mut self, w: &mut W, data: T) -> DicWriteResult<usize> {
        let str_data: &str = data.as_ref();
        if str_data.len() > 4 * 64 * 1024 {
            return Err(InvalidSize {
                actual: str_data.len(),
                expected: 4 * 64 * 1024,
            });
        }

        let mut scratch: [u16; 2] = [0; 2];
        let mut length: usize = 0;
        self.buffer.clear();

        for c in str_data.chars() {
            for u16c in c.encode_utf16(&mut scratch) {
                self.buffer.extend_from_slice(&u16c.to_le_bytes());
                length += 1;
            }
        }

        let prefix = self.write_len(w, length)?;
        w.write_all(&self.buffer)?;
        Ok(prefix + self.buffer.len())
    }

    pub fn write_empty_if_equal<W, T1, T2>(
        &mut self,
        w: &mut W,
        data: T1,
        other: T2,
    ) -> DicWriteResult<usize>
    where
        W: Write,
        T1: AsRef<str> + PartialEq<T2>,
    {
        if data == other {
            self.write(w, "")
        } else {
            self.write(w, data)
        }
    }
}

pub(crate) trait ToU32 {
    fn to_u32(&self) -> u32;
}

impl ToU32 for u32 {
    fn to_u32(&self) -> u32 {
        *self
    }
}

impl ToU32 for i32 {
    fn to_u32(&self) -> u32 {
        *self as u32
    }
}

impl ToU32 for WordId {
    fn to_u32(&self) -> u32 {
        self.as_raw()
    }
}

impl ToU32 for SplitUnit {
    fn to_u32(&self) -> u32 {
        match self {
            SplitUnit::Ref(w) => w.to_u32(),
            SplitUnit::Inline { .. } => panic!("splits must be resolved before writing"),
        }
    }
}

pub(crate) fn write_u32_array<W: Write, T: ToU32>(w: &mut W, data: &[T]) -> DicWriteResult<usize> {
    let len = data.len();
    if len > 127 {
        return Err(InvalidSize {
            expected: 127,
            actual: len,
        });
    }
    w.write_all(&[len as u8])?;
    let mut written = 1;

    for o in data {
        let i = o.to_u32();
        w.write_all(&i.to_le_bytes())?;
        written += 4;
    }

    Ok(written)
}

#[cfg(test)]
mod test {
    use crate::dic::build::error::DicWriteResult;
    use crate::dic::build::primitives::{write_u32_array, Utf16Writer};
    use crate::dic::read::u16str::utf16_string_parser;
    use crate::dic::read::u32_array_parser;
    use claim::assert_matches;

    #[test]
    fn write_utf16() {
        let mut writer = Utf16Writer::new();
        let mut data: Vec<u8> = Vec::new();
        writer
            .write(&mut data, "これはテスト文です")
            .expect("success");
        let (remaining, parsed) = utf16_string_parser(&data).expect("parsed");
        assert_eq!(0, remaining.len());
        assert_eq!("これはテスト文です", parsed);
    }

    #[test]
    fn write_strings() -> DicWriteResult<()> {
        let mut writer = Utf16Writer::new();
        let mut data: Vec<u8> = Vec::new();

        let xstr = "";
        let mut w = writer.write(&mut data, xstr)?;
        assert_eq!(data.len(), w);
        let ystr = "あ𠮟";
        w += writer.write(&mut data, ystr)?;
        assert_eq!(data.len(), w);
        let zstr = "0123456789".repeat(15); // > 127 symbols
        w += writer.write(&mut data, &zstr)?;
        assert_eq!(data.len(), w);
        let (rem, parsed) = utf16_string_parser(&data).expect("ok");
        assert_eq!(parsed, xstr);
        let (rem, parsed) = utf16_string_parser(rem).expect("ok");
        assert_eq!(parsed, ystr);
        let (rem, parsed) = utf16_string_parser(rem).expect("ok");
        assert_eq!(parsed, zstr);
        assert_eq!(rem.len(), 0);

        Ok(())
    }

    #[test]
    fn write_ints_empty() {
        let mut data: Vec<u8> = Vec::new();
        let written = write_u32_array(&mut data, &[0u32; 0]).expect("ok");
        assert_eq!(written, 1);
        assert_eq!(data, b"\0");
    }

    #[test]
    fn write_ints_full() {
        let mut data: Vec<u8> = Vec::new();
        let array = [0, 5, u32::MAX, u32::MIN];
        let written = write_u32_array(&mut data, &array).expect("ok");
        let (rem, parsed) = u32_array_parser(&data).expect("ok");
        assert_eq!(rem, b"");
        assert_eq!(parsed, array);
        assert_eq!(written, 4 * 4 + 1);
    }

    #[test]
    fn write_ints_over_length() {
        let mut data: Vec<u8> = Vec::new();
        let array = [0u32; 130];
        let status = write_u32_array(&mut data, &array);
        assert_matches!(status, Err(_));
    }
}
