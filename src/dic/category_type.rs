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
