/*
 * Copyright (c) 2025-2026 Works Applications Co., Ltd.
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use nom::number::complete::le_i16;

use crate::error::SudachiResult;

use super::read::utf16_string::utf16_string;

pub const POS_DEPTH: usize = 6;

/// A part of speech
///
/// Its length must be `POS_DEPTH`
type POS = Vec<String>;

#[derive(Clone, Debug, Default)]
pub struct PosList(Vec<POS>);

impl From<PosList> for Vec<POS> {
    fn from(val: PosList) -> Self {
        val.0
    }
}

impl std::ops::Deref for PosList {
    type Target = Vec<POS>;
    fn deref(&self) -> &Vec<POS> {
        &self.0
    }
}

impl std::ops::DerefMut for PosList {
    fn deref_mut(&mut self) -> &mut Vec<POS> {
        &mut self.0
    }
}

impl IntoIterator for PosList {
    type Item = POS;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl PosList {
    pub fn from_bytes(buf: &[u8]) -> SudachiResult<Self> {
        let (rest, num_pos) = le_i16(buf)?;
        let (_rest, pos_list) =
            nom::multi::count(nom::multi::count(utf16_string, POS_DEPTH), num_pos as usize)(rest)?;
        Ok(PosList(pos_list))
    }
}
