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

#[derive(Copy, Clone, Eq, PartialEq, Default)]
#[repr(transparent)]
pub struct CreatedWords(Carrier);

impl CreatedWords {
    const MAX_VALUE: Carrier = 63;

    pub fn empty() -> CreatedWords {
        return Default::default();
    }

    pub fn single<Pos: Into<i64>>(position: Pos) -> CreatedWords {
        let raw = position.into();
        debug_assert!(raw > 0);
        let raw = raw as Carrier;
        let shift = min(raw.saturating_sub(1), CreatedWords::MAX_VALUE);
        let bits = (1 as Carrier) << shift;
        CreatedWords(bits)
    }

    #[must_use]
    pub fn add_word<P: Into<i64>>(&self, position: P) -> CreatedWords {
        let mask = CreatedWords::single(position);
        return self.add(mask);
    }

    #[must_use]
    pub fn add(&self, other: CreatedWords) -> CreatedWords {
        CreatedWords(self.0 | other.0)
    }

    pub fn has_word<P: Into<i64>>(&self, position: P) -> bool {
        let mask = CreatedWords::single(position);
        return (self.0 & mask.0) != 0;
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
        assert!(mask.has_word(1));
    }

    #[test]
    fn add() {
        let mask1 = CreatedWords::single(5);
        let mask2 = mask1.add_word(10);
        assert!(mask2.has_word(5));
        assert!(mask2.has_word(10));
        assert!(!mask2.has_word(15));
    }
}
