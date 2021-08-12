use std::collections::HashMap;

#[derive(Debug, Eq, PartialEq)]
pub enum Error {
    NONE,
    POINT,
    COMMA,
    OTHER,
}

#[derive(Debug)]
pub struct NumericParser {
    char_to_num: HashMap<char, i32>,
    digit_length: usize,
    is_first_digit: bool,
    has_comma: bool,
    has_hanging_point: bool,
    pub error_state: Error,
    total: StringNumber,
    subtotal: StringNumber,
    tmp: StringNumber,
}

impl NumericParser {
    pub fn new() -> NumericParser {
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
        let char_to_num: HashMap<_, _> = char_to_num_data
            .iter()
            .map(|(k, v)| (*k, *v))
            .chain((0..10).map(|i| (i.to_string().chars().next().unwrap(), i)))
            .collect();

        NumericParser {
            char_to_num,
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

        let n = match self.char_to_num.get(c) {
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

#[derive(Debug)]
struct StringNumber {
    significand: String,
    scale: usize,
    point: i32,
    is_all_zero: bool,
}

impl StringNumber {
    fn new() -> StringNumber {
        StringNumber {
            significand: String::new(),
            scale: 0,
            point: -1,
            is_all_zero: true,
        }
    }

    fn clear(&mut self) {
        self.significand.clear();
        self.scale = 0;
        self.point = -1;
        self.is_all_zero = true;
    }

    fn append(&mut self, i: i32) {
        if i != 0 {
            self.is_all_zero = false;
        }
        self.significand += &i.to_string();
    }

    fn shift_scale(&mut self, i: i32) {
        if self.is_zero() {
            self.significand += "1";
        }
        self.scale = (self.scale as i32 + i) as usize;
    }

    fn add(&mut self, number: &mut StringNumber) -> bool {
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

    fn set_point(&mut self) -> bool {
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

    fn is_zero(&self) -> bool {
        self.significand.len() == 0
    }

    fn to_string(&mut self) -> String {
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
