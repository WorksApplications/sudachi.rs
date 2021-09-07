/*
 * Copyright (c) 2021 Works Applications Co., Ltd.
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

#[derive(Debug)]
pub struct StringNumber {
    significand: String,
    scale: usize,
    point: i32,
    pub is_all_zero: bool,
}

impl StringNumber {
    pub fn new() -> StringNumber {
        StringNumber {
            significand: String::new(),
            scale: 0,
            point: -1,
            is_all_zero: true,
        }
    }

    pub fn clear(&mut self) {
        self.significand.clear();
        self.scale = 0;
        self.point = -1;
        self.is_all_zero = true;
    }

    pub fn append(&mut self, i: i32) {
        if i != 0 {
            self.is_all_zero = false;
        }
        self.significand += &i.to_string();
    }

    pub fn shift_scale(&mut self, i: i32) {
        if self.is_zero() {
            self.significand += "1";
        }
        self.scale = (self.scale as i32 + i) as usize;
    }

    pub fn add(&mut self, number: &mut StringNumber) -> bool {
        if number.is_zero() {
            return true;
        }

        if self.is_zero() {
            self.significand += &number.significand;
            self.scale = number.scale;
            self.point = number.point;
            return true;
        }

        self.normalize_scale();
        let length = number.int_length();
        if self.scale >= length {
            self.fill_zero(self.scale - length);
            if number.point >= 0 {
                self.point = self.significand.len() as i32 + number.point;
            }
            self.significand += &number.significand;
            self.scale = number.scale;
            return true;
        }

        false
    }

    pub fn set_point(&mut self) -> bool {
        if self.scale == 0 && self.point < 0 {
            self.point = self.significand.len() as i32;
            return true;
        }
        false
    }

    fn int_length(&mut self) -> usize {
        self.normalize_scale();
        if self.point >= 0 {
            return self.point as usize;
        }
        self.significand.len() + self.scale
    }

    pub fn is_zero(&self) -> bool {
        self.significand.len() == 0
    }

    pub fn to_string(&mut self) -> String {
        if self.is_zero() {
            return "0".to_owned();
        }

        self.normalize_scale();
        if self.scale > 0 {
            self.fill_zero(self.scale);
        } else if self.point >= 0 {
            self.significand.insert(self.point as usize, '.');
            if self.point == 0 {
                self.significand.insert(0, '0');
            }
            let n_last_zero = self
                .significand
                .chars()
                .rev()
                .take_while(|c| *c == '0')
                .count();
            self.significand
                .truncate(self.significand.len() - n_last_zero);
            if self.significand.chars().last().unwrap() == '.' {
                self.significand.truncate(self.significand.len() - 1);
            }
        }

        self.significand.clone()
    }

    fn normalize_scale(&mut self) {
        if self.point >= 0 {
            let n_scale = self.significand.len() as i32 - self.point;
            if n_scale > self.scale as i32 {
                self.point += self.scale as i32;
                self.scale = 0;
            } else {
                self.scale -= n_scale as usize;
                self.point = -1;
            }
        }
    }

    fn fill_zero(&mut self, length: usize) {
        self.significand += &"0".repeat(length);
    }
}
