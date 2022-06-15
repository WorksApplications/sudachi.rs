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

pub trait CheckParams {
    fn check_left_id<T: Into<i64>>(&self, raw: T) -> SudachiResult<u16>;
    fn check_right_id<T: Into<i64>>(&self, raw: T) -> SudachiResult<u16>;
    fn check_cost<T: Into<i64>>(&self, raw: T) -> SudachiResult<i16>;
}

impl<'a> CheckParams for Grammar<'a> {
    fn check_left_id<T: Into<i64>>(&self, raw: T) -> SudachiResult<u16> {
        let x = raw.into();
        if x < 0 {
            return Err(SudachiError::InvalidDataFormat(
                0,
                format!("leftId was negative ({}), it must be positive", x),
            ));
        }
        let ux = x as usize;
        if ux > self.conn_matrix().num_left() {
            return Err(SudachiError::InvalidDataFormat(
                ux,
                format!("max grammar leftId is {}", self.conn_matrix().num_left()),
            ));
        }
        return Ok(x as u16);
    }

    fn check_right_id<T: Into<i64>>(&self, raw: T) -> SudachiResult<u16> {
        let x = raw.into();
        if x < 0 {
            return Err(SudachiError::InvalidDataFormat(
                0,
                format!("rightId was negative ({}), it must be positive", x),
            ));
        }
        let ux = x as usize;
        if ux > self.conn_matrix().num_right() {
            return Err(SudachiError::InvalidDataFormat(
                ux,
                format!("max grammar rightId is {}", self.conn_matrix().num_right()),
            ));
        }
        return Ok(x as u16);
    }

    fn check_cost<T: Into<i64>>(&self, raw: T) -> SudachiResult<i16> {
        let x = raw.into();
        if x < (i16::MIN as i64) {
            return Err(SudachiError::InvalidDataFormat(
                0,
                format!(
                    "cost ({}) was lower than the lowest acceptable value ({})",
                    x,
                    i16::MIN
                ),
            ));
        }
        if x > (i16::MAX as i64) {
            return Err(SudachiError::InvalidDataFormat(
                0,
                format!(
                    "cost ({}) was higher than highest acceptable value ({})",
                    x,
                    i16::MAX
                ),
            ));
        }
        return Ok(x as i16);
    }
}
