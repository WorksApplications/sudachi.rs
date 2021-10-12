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

use lazy_static::lazy_static;
use std::collections::HashMap;

mod string_number;
use crate::hash::RoMu;
use string_number::StringNumber;

/// State of the parser
#[derive(Debug, Eq, PartialEq)]
pub enum Error {
    NONE,
    POINT,
    COMMA,
    // OTHER,
}

/// Parses number written by arabic or kanji
#[derive(Debug)]
pub struct NumericParser {
    digit_length: usize,
    is_first_digit: bool,
    has_comma: bool,
    has_hanging_point: bool,
    pub error_state: Error,
    total: StringNumber,
    subtotal: StringNumber,
    tmp: StringNumber,
}

lazy_static! {
    static ref CHAR_TO_NUM: HashMap<char, i32, RoMu> = make_char_to_num_data();
}

fn make_char_to_num_data() -> HashMap<char, i32, RoMu> {
    let char_to_num_data = [
        ('〇', 0),
        ('一', 1),
        ('二', 2),
        ('三', 3),
        ('四', 4),
        ('五', 5),
        ('六', 6),
        ('七', 7),
        ('八', 8),
        ('九', 9),
        ('十', -1),
        ('百', -2),
        ('千', -3),
        ('万', -4),
        ('億', -8),
        ('兆', -12),
    ];

    let char_to_num: HashMap<_, _, RoMu> = char_to_num_data
        .iter()
        .map(|(k, v)| (*k, *v))
        .chain((0..10).map(|i| (i.to_string().chars().next().unwrap(), i)))
        .collect();

    char_to_num
}

impl NumericParser {
    pub fn new() -> NumericParser {
        NumericParser {
            digit_length: 0,
            is_first_digit: true,
            has_comma: false,
            has_hanging_point: false,
            error_state: Error::NONE,
            total: StringNumber::new(),
            subtotal: StringNumber::new(),
            tmp: StringNumber::new(),
        }
    }

    pub fn clear(&mut self) {
        self.digit_length = 0;
        self.is_first_digit = true;
        self.has_comma = false;
        self.has_hanging_point = false;
        self.error_state = Error::NONE;
        self.total.clear();
        self.subtotal.clear();
        self.tmp.clear();
    }

    pub fn append(&mut self, c: &char) -> bool {
        if *c == '.' {
            self.has_hanging_point = true;
            if self.is_first_digit {
                self.error_state = Error::POINT;
                return false;
            }
            if self.has_comma && !self.check_comma() {
                self.error_state = Error::COMMA;
                return false;
            }
            if !self.tmp.set_point() {
                self.error_state = Error::POINT;
                return false;
            }
            self.has_comma = false;
            return true;
        }
        if *c == ',' {
            if !self.check_comma() {
                self.error_state = Error::COMMA;
                return false;
            }
            self.has_comma = true;
            self.digit_length = 0;
            return true;
        }

        let n = match CHAR_TO_NUM.get(c) {
            None => return false,
            Some(v) => *v,
        };
        if NumericParser::is_small_unit(n) {
            self.tmp.shift_scale(-n);
            if !self.subtotal.add(&mut self.tmp) {
                return false;
            }
            self.tmp.clear();
            self.is_first_digit = true;
            self.digit_length = 0;
            self.has_comma = false;
        } else if NumericParser::is_large_unit(n) {
            if !self.subtotal.add(&mut self.tmp) || self.subtotal.is_zero() {
                return false;
            }
            self.subtotal.shift_scale(-n);
            if !self.total.add(&mut self.subtotal) {
                return false;
            }
            self.subtotal.clear();
            self.tmp.clear();
            self.is_first_digit = true;
            self.digit_length = 0;
            self.has_comma = false;
        } else {
            self.tmp.append(n);
            self.is_first_digit = false;
            self.digit_length += 1;
            self.has_hanging_point = false;
        }

        true
    }

    pub fn done(&mut self) -> bool {
        let ret = self.subtotal.add(&mut self.tmp) && self.total.add(&mut self.subtotal);
        if self.has_hanging_point {
            self.error_state = Error::POINT;
            return false;
        }
        if self.has_comma && self.digit_length != 3 {
            self.error_state = Error::COMMA;
            return false;
        }
        ret
    }

    pub fn get_normalized(&mut self) -> String {
        self.total.to_string()
    }

    fn check_comma(&self) -> bool {
        if self.is_first_digit {
            return false;
        }
        if !self.has_comma {
            return self.digit_length <= 3 && !self.tmp.is_zero() && !self.tmp.is_all_zero;
        }
        self.digit_length == 3
    }

    fn is_small_unit(n: i32) -> bool {
        -3 <= n && n < 0
    }
    fn is_large_unit(n: i32) -> bool {
        n < -3
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(parser: &mut NumericParser, text: &str) -> bool {
        for c in text.to_string().chars() {
            if !parser.append(&c) {
                return false;
            }
        }
        parser.done()
    }

    #[test]
    fn digits() {
        let mut parser = NumericParser::new();
        assert!(parse(&mut parser, "1000"));
        assert_eq!("1000", parser.get_normalized());
    }

    #[test]
    fn starts_with_zero() {
        let mut parser = NumericParser::new();
        assert!(parse(&mut parser, "001000"));
        assert_eq!("001000", parser.get_normalized());

        parser.clear();
        assert!(parse(&mut parser, "〇一〇〇〇"));
        assert_eq!("01000", parser.get_normalized());

        parser.clear();
        assert!(parse(&mut parser, "00.1000"));
        assert_eq!("00.1", parser.get_normalized());

        parser.clear();
        assert!(parse(&mut parser, "000"));
        assert_eq!("000", parser.get_normalized());
    }

    #[test]
    fn use_small_unit() {
        let mut parser = NumericParser::new();
        assert!(parse(&mut parser, "二十七"));
        assert_eq!("27", parser.get_normalized());

        parser.clear();
        assert!(parse(&mut parser, "千三百二十七"));
        assert_eq!("1327", parser.get_normalized());

        parser.clear();
        assert!(parse(&mut parser, "千十七"));
        assert_eq!("1017", parser.get_normalized());

        parser.clear();
        assert!(parse(&mut parser, "千三百二十七.〇五"));
        assert_eq!("1327.05", parser.get_normalized());

        parser.clear();
        assert!(!parse(&mut parser, "三百二十百"));
    }

    #[test]
    fn use_large_unit() {
        let mut parser = NumericParser::new();
        assert!(parse(&mut parser, "1万"));
        assert_eq!("10000", parser.get_normalized());

        parser.clear();
        assert!(parse(&mut parser, "千三百二十七万"));
        assert_eq!("13270000", parser.get_normalized());

        parser.clear();
        assert!(parse(&mut parser, "千三百二十七万一四"));
        assert_eq!("13270014", parser.get_normalized());

        parser.clear();
        assert!(parse(&mut parser, "千三百二十七万一四.〇五"));
        assert_eq!("13270014.05", parser.get_normalized());

        parser.clear();
        assert!(parse(&mut parser, "三兆2千億千三百二十七万一四.〇五"));
        assert_eq!("3200013270014.05", parser.get_normalized());

        parser.clear();
        assert!(!parse(&mut parser, "億万"));
    }

    #[test]
    fn float_with_unit() {
        let mut parser = NumericParser::new();
        assert!(parse(&mut parser, "1.5千"));
        assert_eq!("1500", parser.get_normalized());

        parser.clear();
        assert!(parse(&mut parser, "1.5百万"));
        assert_eq!("1500000", parser.get_normalized());

        parser.clear();
        assert!(parse(&mut parser, "1.5百万1.5千20"));
        assert_eq!("1501520", parser.get_normalized());

        parser.clear();
        assert!(!parse(&mut parser, "1.5千5百"));

        parser.clear();
        assert!(!parse(&mut parser, "1.5千500"));
    }

    #[test]
    fn log_numeric() {
        let mut parser = NumericParser::new();
        assert!(parse(&mut parser, "200000000000000000000万"));
        assert_eq!("2000000000000000000000000", parser.get_normalized());
    }

    #[test]
    fn with_comma() {
        let mut parser = NumericParser::new();
        assert!(parse(&mut parser, "2,000,000"));
        assert_eq!("2000000", parser.get_normalized());

        parser.clear();
        assert!(parse(&mut parser, "259万2,300"));
        assert_eq!("2592300", parser.get_normalized());

        parser.clear();
        assert!(!parse(&mut parser, "200,00,000"));
        assert_eq!(Error::COMMA, parser.error_state);

        parser.clear();
        assert!(!parse(&mut parser, "2,4"));
        assert_eq!(Error::COMMA, parser.error_state);

        parser.clear();
        assert!(!parse(&mut parser, "000,000"));
        assert_eq!(Error::COMMA, parser.error_state);

        parser.clear();
        assert!(!parse(&mut parser, ",000"));
        assert_eq!(Error::COMMA, parser.error_state);

        parser.clear();
        assert!(!parse(&mut parser, "256,55.1"));
        assert_eq!(Error::COMMA, parser.error_state);
    }

    #[test]
    fn not_digit() {
        let mut parser = NumericParser::new();
        assert!(!parse(&mut parser, "@@@"));
    }

    #[test]
    fn float_point() {
        let mut parser = NumericParser::new();
        assert!(parse(&mut parser, "6.0"));
        assert_eq!("6", parser.get_normalized());

        parser.clear();
        assert!(!parse(&mut parser, "6."));
        assert_eq!(Error::POINT, parser.error_state);

        parser.clear();
        assert!(!parse(&mut parser, "1.2.3"));
        assert_eq!(Error::POINT, parser.error_state);
    }
}
