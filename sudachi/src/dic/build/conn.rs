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

use std::fs::File;
use std::io::{BufReader, Write};
use std::path::Path;

use lazy_static::lazy_static;
use regex::Regex;

use crate::dic::build::error::{BuildFailure, DicBuildError, DicCompilationCtx, DicWriteResult};
use crate::dic::build::parse::{it_next, parse_i16};
use crate::error::SudachiResult;

pub struct ConnBuffer {
    matrix: Vec<u8>,
    ctx: DicCompilationCtx,
    line: String,
    num_left: i16,
    num_right: i16,
}

lazy_static! {
    static ref SPLIT_REGEX: Regex = Regex::new(r"\s+").unwrap();
    static ref EMPTY_LINE: Regex = Regex::new(r"^\s*$").unwrap();
}

impl ConnBuffer {
    pub fn new() -> Self {
        Self {
            matrix: Vec::new(),
            ctx: DicCompilationCtx::default(),
            line: String::new(),
            num_left: 0,
            num_right: 0,
        }
    }

    #[allow(unused)]
    pub fn matrix(&self) -> &[u8] {
        &self.matrix
    }

    #[allow(unused)]
    pub fn left(&self) -> i16 {
        self.num_left
    }

    #[allow(unused)]
    pub fn right(&self) -> i16 {
        self.num_right
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> SudachiResult<usize> {
        if self.num_left < 0 {
            return num_error("left", self.num_left);
        }

        if self.num_right < 0 {
            return num_error("right", self.num_right);
        }

        writer.write_all(&i16::to_le_bytes(self.num_left))?;
        writer.write_all(&i16::to_le_bytes(self.num_right))?;
        writer.write_all(&self.matrix)?;
        Ok(4 + self.matrix.len())
    }

    pub fn read_file(&mut self, path: &Path) -> SudachiResult<()> {
        let file = File::open(path)?;
        let bufrd = BufReader::with_capacity(32 * 1024, file);
        let filename = path.to_str().unwrap_or("unknown").to_owned();
        let old = self.ctx.set_filename(filename);
        let status = self.read(bufrd);
        self.ctx.set_filename(old);
        status
    }

    pub fn read<R: std::io::BufRead>(&mut self, mut reader: R) -> SudachiResult<()> {
        self.ctx.set_line(0);
        loop {
            let nread = reader.read_line(&mut self.line)?;
            if nread == 0 {
                todo!()
            }
            self.ctx.add_line(1);
            if !EMPTY_LINE.is_match(&self.line) {
                break;
            }
        }

        let result = self.parse_header();
        let (left, right) = self.ctx.transform(result)?;
        if left < 0 {
            return num_error("left", left);
        }

        if right < 0 {
            return num_error("right", right);
        }

        let size = left as usize * right as usize * 2;
        self.matrix.resize(size, 0);
        self.num_left = left;
        self.num_right = right;

        loop {
            self.line.clear();
            let nread = reader.read_line(&mut self.line)?;
            if nread == 0 {
                break;
            }
            self.ctx.add_line(1);

            if EMPTY_LINE.is_match(&self.line) {
                continue;
            }

            // borrow checker complains when written as a single line
            let status = self.parse_line();
            self.ctx.transform(status)?;
        }

        Ok(())
    }

    fn parse_header(&mut self) -> DicWriteResult<(i16, i16)> {
        let mut items = SPLIT_REGEX.splitn(&self.line.trim(), 2);
        // TODO: fix get_next error message
        let left = it_next(&self.line, &mut items, "left_num", parse_i16)?;
        let right = it_next(&self.line, &mut items, "right_num", parse_i16)?;
        Ok((left, right))
    }

    fn parse_line(&mut self) -> DicWriteResult<()> {
        let mut items = SPLIT_REGEX.splitn(&self.line.trim(), 3);
        let left = it_next(&self.line, &mut items, "left", parse_i16)?;
        let right = it_next(&self.line, &mut items, "right", parse_i16)?;
        let cost = it_next(&self.line, &mut items, "cost", parse_i16)?;
        self.write_elem(left, right, cost)
    }

    fn write_elem(&mut self, left: i16, right: i16, cost: i16) -> DicWriteResult<()> {
        let index = right as usize * self.num_left as usize + left as usize;
        let index = index * 2;
        let bytes = cost.to_le_bytes();
        self.matrix[index] = bytes[0];
        self.matrix[index + 1] = bytes[1];
        Ok(())
    }
}

fn num_error<T>(part: &'static str, value: i16) -> SudachiResult<T> {
    return Err(DicBuildError {
        file: "<connection>".to_owned(),
        line: 0,
        cause: BuildFailure::InvalidConnSize(part, value),
    }
    .into());
}

#[cfg(test)]
mod test {
    use crate::dic::build::conn::ConnBuffer;
    use crate::dic::connect::ConnectionMatrix;

    #[test]
    fn parse_simple2x2() {
        let data = "
        2 2
        0 0 0
        0 1 1
        1 0 2
        1 1 3";
        let mut parser = ConnBuffer::new();
        parser.read(data.as_bytes()).unwrap();
        let cost = ConnectionMatrix::from_offset_size(
            parser.matrix(),
            0,
            parser.left() as _,
            parser.right() as _,
        )
        .unwrap();
        assert_eq!(cost.cost(0, 0), 0);
        assert_eq!(cost.cost(0, 1), 1);
        assert_eq!(cost.cost(1, 0), 2);
        assert_eq!(cost.cost(1, 1), 3);
    }
}
