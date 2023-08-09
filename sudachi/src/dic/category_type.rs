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

use crate::error::SudachiError;
use bitflags::bitflags;
use std::fmt::{Debug, Formatter};
use std::str::FromStr;

bitflags! {
    /// A set of categories for a character
    ///
    /// Implemented as a bitset with fixed size
    #[repr(transparent)]
    #[derive(Clone, Copy, PartialEq, Eq, Hash)]
    pub struct CategoryType: u32 {
        /** The fall back category. */
        const DEFAULT = (1 << 0);
        /** White spaces. */
        const SPACE = (1 << 1);
        /** CJKV ideographic characters. */
        const KANJI = (1 << 2);
        /** Symbols. */
        const SYMBOL = (1 << 3);
        /** Numerical characters. */
        const NUMERIC = (1 << 4);
        /** Latin alphabets. */
        const ALPHA = (1 << 5);
        /** Hiragana characters. */
        const HIRAGANA = (1 << 6);
        /** Katakana characters. */
        const KATAKANA = (1 << 7);
        /** Kanji numeric characters. */
        const KANJINUMERIC = (1 << 8);
        /** Greek alphabets. */
        const GREEK = (1 << 9);
        /** Cyrillic alphabets. */
        const CYRILLIC = (1 << 10);
        /** User defined category. */
        const USER1 = (1 << 11);
        /** User defined category. */
        const USER2 = (1 << 12);
        /** User defined category. */
        const USER3 = (1 << 13);
        /** User defined category. */
        const USER4 = (1 << 14);
        /** This character cannot be the beginning of an OOV word */
        const NOOOVBOW = (1 << 30);
        /** This and next characters cannot be the beginning of an OOV word */
        const NOOOVBOW2 = (1 << 31);

        /** All categories at once except NOOOVBOW/2 */
        const ALL = 0b00111111_11111111_11111111_11111111;
    }
}

impl CategoryType {
    pub fn count(self) -> u32 {
        self.bits().count_ones()
    }
}

impl Default for CategoryType {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl Debug for CategoryType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        bitflags::parser::to_writer(self, f)
    }
}

impl FromStr for CategoryType {
    type Err = SudachiError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let result = bitflags::parser::from_str::<CategoryType>(s);
        result.map_err(|_| SudachiError::InvalidCharacterCategoryType(s.into()))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use claim::assert_matches;

    #[test]
    fn format() {
        assert_eq!("GREEK", format!("{:?}", CategoryType::GREEK));
        assert_eq!(
            "SPACE | GREEK",
            format!("{:?}", CategoryType::GREEK | CategoryType::SPACE)
        );
    }

    #[test]
    fn ct_size() {
        assert_eq!(4, std::mem::size_of::<CategoryType>())
    }

    #[test]
    fn count() {
        let c1 = CategoryType::GREEK | CategoryType::KANJI;
        assert_eq!(c1.count(), 2);
        let c2 = CategoryType::USER1 | CategoryType::USER2 | CategoryType::USER3;
        assert_eq!(c2.count(), 3);
    }

    #[test]
    fn iter_is_correct_3() {
        let ct = CategoryType::GREEK | CategoryType::KANJI | CategoryType::USER1;
        let mut iter = ct.iter();
        assert_matches!(iter.next(), Some(CategoryType::KANJI));
        assert_matches!(iter.next(), Some(CategoryType::GREEK));
        assert_matches!(iter.next(), Some(CategoryType::USER1));
        assert_matches!(iter.next(), None);
    }
}
