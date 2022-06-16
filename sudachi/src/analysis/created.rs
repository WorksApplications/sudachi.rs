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

use std::cmp::min;

type Carrier = u64;

/// Bitset which represents that a word of a specified length was created.
/// Lattice construction fills this bitmap and passes it to the OOV providers.
/// It allows OOV providers to check if a word of a specific length was created very cheaply.
///
/// Unfortunately, if a word is more than `MAX_VALUE` characters, handlers need to do usual linear-time check.
#[derive(Copy, Clone, Eq, PartialEq, Default, Debug)]
#[repr(transparent)]
pub struct CreatedWords(Carrier);

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum HasWord {
    Yes,
    No,
    Maybe,
}

impl CreatedWords {
    /// Maximum supported length of the word
    pub const MAX_VALUE: Carrier = 64;
    const MAX_SHIFT: Carrier = CreatedWords::MAX_VALUE - 1;

    pub fn empty() -> CreatedWords {
        return Default::default();
    }

    pub fn single<Pos: Into<i64>>(length: Pos) -> CreatedWords {
        let raw = length.into();
        debug_assert!(raw > 0);
        let raw = raw as Carrier;
        let shift = min(raw.saturating_sub(1), CreatedWords::MAX_SHIFT);
        let bits = (1 as Carrier) << shift;
        CreatedWords(bits)
    }

    #[must_use]
    pub fn add_word<P: Into<i64>>(&self, length: P) -> CreatedWords {
        let mask = CreatedWords::single(length);
        return self.add(mask);
    }

    #[must_use]
    pub fn add(&self, other: CreatedWords) -> CreatedWords {
        CreatedWords(self.0 | other.0)
    }

    pub fn has_word<P: Into<i64> + Copy>(&self, length: P) -> HasWord {
        let mask = CreatedWords::single(length);
        if (self.0 & mask.0) == 0 {
            HasWord::No
        } else {
            if length.into() >= CreatedWords::MAX_VALUE as _ {
                HasWord::Maybe
            } else {
                HasWord::Yes
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        return self.0 == 0;
    }

    pub fn not_empty(&self) -> bool {
        return !self.is_empty();
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn simple() {
        let mask = CreatedWords::single(1);
        assert_eq!(mask.has_word(1), HasWord::Yes);
    }

    #[test]
    fn add() {
        let mask1 = CreatedWords::single(5);
        let mask2 = mask1.add_word(10);
        assert_eq!(mask2.has_word(5), HasWord::Yes);
        assert_eq!(mask2.has_word(10), HasWord::Yes);
        assert_eq!(mask2.has_word(15), HasWord::No);
    }

    #[test]
    fn long_value_present() {
        let mask1 = CreatedWords::single(100);
        assert_eq!(HasWord::No, mask1.has_word(62));
        assert_eq!(HasWord::No, mask1.has_word(63));
        assert_eq!(HasWord::Maybe, mask1.has_word(64));
    }

    #[test]
    fn long_value_absent() {
        let mask1 = CreatedWords::single(62);
        assert_eq!(HasWord::Yes, mask1.has_word(62));
        assert_eq!(HasWord::No, mask1.has_word(63));
        assert_eq!(HasWord::No, mask1.has_word(64));
    }
}
