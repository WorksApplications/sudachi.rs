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

use crate::dic::grammar::Grammar;
use crate::error::{SudachiError, SudachiResult};
use itertools::Itertools;
use serde::Deserialize;
use std::fmt::Display;

#[derive(Eq, PartialEq, Deserialize, Clone, Copy, Debug)]
#[serde(rename_all = "lowercase")]
pub enum UserPosMode {
    Allow,
    Forbid,
}

impl Default for UserPosMode {
    fn default() -> Self {
        UserPosMode::Forbid
    }
}

pub trait UserPosSupport {
    fn handle_user_pos<S: AsRef<str> + ToString + Display>(
        &mut self,
        pos: &[S],
        mode: UserPosMode,
    ) -> SudachiResult<u16>;
}

impl<'a> UserPosSupport for &'a mut Grammar<'_> {
    fn handle_user_pos<S: AsRef<str> + ToString + Display>(
        &mut self,
        pos: &[S],
        mode: UserPosMode,
    ) -> SudachiResult<u16> {
        if let Some(id) = self.get_part_of_speech_id(pos) {
            return Ok(id);
        }

        match mode {
            UserPosMode::Allow => self.register_pos(pos),
            UserPosMode::Forbid => Err(SudachiError::InvalidPartOfSpeech(format!(
                "POS {} was not in the dictionary, user-defined POS are forbidden",
                pos.iter().join(",")
            ))),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn allow() {
        let mode: UserPosMode = serde_json::from_str("\"allow\"").expect("fails");
        assert_eq!(UserPosMode::Allow, mode)
    }

    #[test]
    fn forbid() {
        let mode: UserPosMode = serde_json::from_str("\"forbid\"").expect("fails");
        assert_eq!(UserPosMode::Forbid, mode)
    }

    #[test]
    fn other_value() {
        let mode: Result<UserPosMode, _> = serde_json::from_str("\"test\"");
        assert!(mode.is_err())
    }
}
