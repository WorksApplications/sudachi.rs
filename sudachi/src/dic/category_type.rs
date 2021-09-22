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

use std::collections::HashSet;
use std::str::FromStr;

use crate::error::SudachiError;

/// Categories of characters.
///
/// You can define the range of each category in the file which specified
/// "characterDefinitionFile" of the settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CategoryType {
    /** The fall back category. */
    DEFAULT,
    /** White spaces. */
    SPACE,
    /** CJKV ideographic characters. */
    KANJI,
    /** Symbols. */
    SYMBOL,
    /** Numerical characters. */
    NUMERIC,
    /** Latin alphabets. */
    ALPHA,
    /** Hiragana characters. */
    HIRAGANA,
    /** Katakana characters. */
    KATAKANA,
    /** Kanji numeric characters. */
    KANJINUMERIC,
    /** Greek alphabets. */
    GREEK,
    /** Cyrillic alphabets. */
    CYRILLIC,
    /** User defined category. */
    USER1,
    /** User defined category. */
    USER2,
    /** User defined category. */
    USER3,
    /** User defined category. */
    USER4,
    /** Characters that cannot be the beginning of word */
    NOOOVBOW,
}

/// Set of category types
pub type CategoryTypes = HashSet<CategoryType>;

impl FromStr for CategoryType {
    type Err = SudachiError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "DEFAULT" => Ok(CategoryType::DEFAULT),
            "SPACE" => Ok(CategoryType::SPACE),
            "KANJI" => Ok(CategoryType::KANJI),
            "SYMBOL" => Ok(CategoryType::SYMBOL),
            "NUMERIC" => Ok(CategoryType::NUMERIC),
            "ALPHA" => Ok(CategoryType::ALPHA),
            "HIRAGANA" => Ok(CategoryType::HIRAGANA),
            "KATAKANA" => Ok(CategoryType::KATAKANA),
            "KANJINUMERIC" => Ok(CategoryType::KANJINUMERIC),
            "GREEK" => Ok(CategoryType::GREEK),
            "CYRILLIC" => Ok(CategoryType::CYRILLIC),
            "USER1" => Ok(CategoryType::USER1),
            "USER2" => Ok(CategoryType::USER2),
            "USER3" => Ok(CategoryType::USER3),
            "USER4" => Ok(CategoryType::USER4),
            "NOOOVBOW" => Ok(CategoryType::NOOOVBOW),
            _ => Err(SudachiError::InvalidCharacterCategoryType(String::from(s))),
        }
    }
}
