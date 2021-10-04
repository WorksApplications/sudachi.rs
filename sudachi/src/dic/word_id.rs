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

use std::fmt::{Debug, Display, Formatter};

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(transparent)]
pub struct WordId {
    raw: u32,
}

impl Debug for WordId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl Display for WordId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let fmtdic = if self.is_oov() { -1 } else { self.dic() as i32 };
        write!(f, "({}, {})", fmtdic, self.word())
    }
}

const WORD_MASK: u32 = 0x0fffffff;

impl WordId {
    pub fn from_raw(raw: u32) -> WordId { WordId { raw } }

    pub fn new(dic: u8, word: u32) -> WordId {
        debug_assert_eq!(word & (!WORD_MASK), 0);
        debug_assert_eq!(dic & (!0xf), 0);
        let dic_part = ((dic & 0xf) as u32) << 28;
        let word_part = word & WORD_MASK;
        let raw = dic_part | word_part;
        return Self::from_raw(raw);
    }

    pub fn dic(&self) -> u8 {
        return (self.raw >> 28) as u8;
    }

    pub fn word(&self) -> u32 {
        return self.raw & WORD_MASK;
    }

    pub fn is_system(&self) -> bool {
        self.dic() == 0
    }

    pub fn is_user(&self) -> bool {
        match self.dic() {
            0 | 0xf => false,
            _ => true
        }
    }

    pub fn is_oov(&self) -> bool {
        self.dic() == 0xf
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn assert_create(dic: u8, word: u32) {
        let id = WordId::new(dic, word);
        assert_eq!(dic, id.dic());
        assert_eq!(word, id.word());
    }

    #[test]
    fn create() {
        assert_create(0, 0);
        assert_create(0, 1);
        assert_create(0, 0x0fffffff);
        assert_create(14, 0x0fffffff);
        assert_create(1, 0);
        assert_create(1, 0x0fffffff);
        assert_create(15, 3121);
        assert_create(15, 0);
        assert_create(15, 0x0fffffff);
    }

    #[test]
    fn display() {
        let id1 = WordId::new(0, 521321);
        assert_eq!("(0, 521321)", format!("{}", id1));
    }

    #[test]
    fn debug() {
        let id1 = WordId::new(0, 521321);
        assert_eq!("(0, 521321)", format!("{:?}", id1));
    }

    #[test]
    fn is_system() {
        assert!(WordId::new(0, 0).is_system());
        assert!(!WordId::new(1, 0).is_system());
        assert!(!WordId::new(14, 0).is_system());
        assert!(!WordId::new(15, 0).is_system());
    }

    #[test]
    fn is_user() {
        assert!(!WordId::new(0, 0).is_user());
        assert!(WordId::new(1, 0).is_user());
        assert!(WordId::new(14, 0).is_user());
        assert!(!WordId::new(15, 0).is_user());
    }

    #[test]
    fn is_oov() {
        assert!(!WordId::new(0, 0).is_oov());
        assert!(!WordId::new(1, 0).is_oov());
        assert!(!WordId::new(14, 0).is_oov());
        assert!(WordId::new(15, 0).is_oov());
    }
}