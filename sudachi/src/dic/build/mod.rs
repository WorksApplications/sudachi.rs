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

use crate::dic::build::error::DicCompilationCtx;
use crate::dic::build::primitives::Utf16Writer;
use std::io::{Seek, Write};

use crate::error::SudachiResult;

pub(crate) mod cost;
pub mod error;
pub(crate) mod primitives;
pub mod read_raw;

const MAX_POS_IDS: usize = i16::MAX as usize;
const MAX_DIC_STRING_LEN: usize = MAX_POS_IDS;
const MAX_ARRAY_LEN: usize = i8::MAX as usize;

struct DictBuilder {
    u16w: Utf16Writer,
    ctx: DicCompilationCtx,
}

impl DictBuilder {
    pub fn new() -> Self {
        DictBuilder {
            u16w: Utf16Writer::new(),
            ctx: DicCompilationCtx::memory(),
        }
    }

    pub fn write_pos_list<W: Write>(
        &mut self,
        data: &Vec<Vec<String>>,
        w: &mut W,
    ) -> SudachiResult<usize> {
        w.write_all(&u64::to_le_bytes(data.len() as u64))?;
        let mut count = 4;
        for row in data {
            for field in row {
                match self.u16w.write(w, field) {
                    Ok(written) => count += written,
                    Err(e) => return self.ctx.err(e),
                }
            }
        }
        Ok(count)
    }
}
