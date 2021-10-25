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

use crate::dic::build::error::DicWriteReason::InvalidSize;
use crate::dic::build::error::DicWriteResult;
use std::io::Write;

use crate::error::SudachiResult;

struct Utf16Writer {
    buffer: Vec<u8>,
}

impl Utf16Writer {
    pub fn new() -> Self {
        Utf16Writer {
            buffer: Vec::with_capacity(256),
        }
    }

    pub fn write<W: Write>(&mut self, w: &mut W, data: &str) -> DicWriteResult<usize> {
        if data.len() > 4 * 64 * 1024 {
            return Err(InvalidSize {
                actual: data.len(),
                expected: 4 * 64 * 1024,
            });
        }

        let mut scratch: [u16; 2] = [0; 2];
        let mut length: usize = 0;
        self.buffer.clear();

        for c in data.chars() {
            for u16c in c.encode_utf16(&mut scratch) {
                self.buffer.extend_from_slice(&u16c.to_le_bytes());
                length += 1;
            }
        }

        if length > i16::MAX as _ {
            return Err(InvalidSize {
                actual: length,
                expected: i16::MAX as _,
            });
        }

        let length = length as u16;

        let prefix = if length < 127 {
            w.write_all(&[length as u8; 1])?;
            1
        } else {
            let b0 = (length as u8) & 0xff;
            let b1 = ((length >> 8) as u8) | 0x80;
            w.write_all(&[b1, b0]);
            2
        };

        w.write_all(&self.buffer)?;
        Ok(prefix + self.buffer.len())
    }
}

#[cfg(test)]
mod test {
    use crate::dic::build::error::DicWriteResult;
    use crate::dic::build::primitives::Utf16Writer;
    use crate::dic::utf16_string_parser;

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
        let l1 = writer.write(&mut data, xstr)?;
        let ystr = "あ𠮟";
        let l2 = writer.write(&mut data, ystr)?;
        let zstr = "0123456789".repeat(15); // > 127 symbols
        let l3 = writer.write(&mut data, &zstr)?;
        let (rem, parsed) = utf16_string_parser(&data).expect("ok");
        assert_eq!(parsed, xstr);
        let (rem, parsed) = utf16_string_parser(rem).expect("ok");
        assert_eq!(parsed, ystr);
        let (rem, parsed) = utf16_string_parser(rem).expect("ok");
        assert_eq!(parsed, zstr);
        assert_eq!(rem.len(), 0);

        Ok(())
    }
}
