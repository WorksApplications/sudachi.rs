/*
 *  Copyright (c) 2021-2026 Works Applications Co., Ltd.
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

use crate::dic::lexicon_set::LexiconSetError;
use crate::error::{SudachiError, SudachiResult};
use std::fmt::{Debug, Display, Formatter};

/// Bit mask for the entry id part of the WordId
const WORD_MASK: u32 = 0x0fff_ffff;

/// Dictionary ID
///
/// Id of the binary dictionary in a combined dictionary.
///
/// 0: system dictionary
/// 1-14: user dictionary
/// 15: OOV and other special nodes
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[repr(transparent)]
pub struct DictId {
    raw: u8,
}

impl DictId {
    /// Create DictId from the compressed representation
    const fn from_raw(raw: u8) -> DictId {
        DictId { raw }
    }

    /// Create a new DictId from parts
    pub fn new(dict: u8) -> DictId {
        debug_assert!(dict <= 15);
        Self::from_raw(dict)
    }

    /// Create a new DictId with correctness checking
    pub fn checked(dict: u8) -> SudachiResult<DictId> {
        if dict > 15 {
            return Err(SudachiError::LexiconSetError(
                LexiconSetError::TooLargeDictionaryId(dict as usize),
            ));
        }
        Ok(DictId::new(dict))
    }

    /// Get the raw value of the DictId
    pub const fn as_raw(&self) -> u8 {
        self.raw
    }

    /// Check if the word comes from the system dictionary
    pub fn is_system(&self) -> bool {
        self.raw == 0
    }

    /// Check if the word comes from the user dictionary
    pub fn is_user(&self) -> bool {
        !matches!(self.raw, 0 | 0xf)
    }

    /// Check if the word is OOV
    /// An OOV node can come of OOV handlers or be a special system node like BOS or EOS
    pub fn is_oov(&self) -> bool {
        self.raw == 0xf
    }

    pub const SYSTEM: Self = DictId::from_raw(0);
    pub const MAX_USER: Self = DictId::from_raw(14);
    pub const SPECIAL: Self = DictId::from_raw(15);
}

/// Entry id
///
/// Id of the entry in a single binary dictionary.
/// Top 4 bits are always 0.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[repr(transparent)]
pub struct EntryId {
    raw: u32,
}

impl EntryId {
    /// Create WordId from the compressed representation
    const fn from_raw(raw: u32) -> Self {
        EntryId { raw }
    }

    /// Create a new EntryId from parts
    pub fn new(entry: u32) -> Self {
        debug_assert_eq!(entry & (!WORD_MASK), 0);
        Self::from_raw(entry)
    }

    /// Create a new EntryId with correctness checking
    pub fn checked(entry: u32) -> SudachiResult<Self> {
        if entry & !WORD_MASK != 0 {
            return Err(SudachiError::LexiconSetError(
                LexiconSetError::TooLargeWordId(entry, WORD_MASK as usize),
            ));
        }
        Ok(Self::new(entry))
    }

    /// Get the raw value of the EntryId
    pub const fn as_raw(&self) -> u32 {
        self.raw
    }

    pub const MAX: u32 = 0x0fff_ffff;
}

/// Dictionary Word ID
///
/// Id of the word in a combined dictionary.
/// Encode dictionary ID and entry ID as 4 bits and 28 bits respectively.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(transparent)]
pub struct WordId {
    raw: u32,
}

impl Default for WordId {
    fn default() -> Self {
        Self::INVALID
    }
}

impl Debug for WordId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl Display for WordId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let fmtdic = if self.is_oov() {
            -1
        } else {
            self.dict().as_raw() as i32
        };
        write!(f, "({}, {})", fmtdic, self.entry().as_raw())
    }
}

impl WordId {
    /// Create WordId from the compressed representation
    pub(crate) const fn from_raw(raw: u32) -> WordId {
        WordId { raw }
    }

    /// Create WordId from Dict and Entry parts.
    pub fn from_parts(dict: DictId, entry: EntryId) -> WordId {
        Self::new(dict.as_raw(), entry.as_raw())
    }

    /// Create WordId from parts
    pub fn new(dict: u8, entry: u32) -> WordId {
        debug_assert_eq!(entry & (!WORD_MASK), 0);
        debug_assert_eq!(dict & (!0xf), 0);
        let dic_part = ((dict & 0xf) as u32) << 28;
        let entry_part = entry & WORD_MASK;
        let raw = dic_part | entry_part;
        Self::from_raw(raw)
    }

    /// Creates the WordId with correctness checking
    pub fn checked(dic: u8, entry: u32) -> SudachiResult<WordId> {
        if dic > 15 {
            return Err(SudachiError::LexiconSetError(
                LexiconSetError::TooLargeDictionaryId(dic as usize),
            ));
        }

        if entry & !WORD_MASK != 0 {
            return Err(SudachiError::LexiconSetError(
                LexiconSetError::TooLargeWordId(entry, WORD_MASK as usize),
            ));
        }

        Ok(Self::new(dic, entry))
    }

    /// Creates an OOV node for pos_id
    pub fn oov(pos_id: u32) -> WordId {
        Self::new(0xf, pos_id)
    }

    /// Extract Dictionary ID
    pub fn dict(&self) -> DictId {
        DictId::new((self.raw >> 28) as u8)
    }

    /// Extract Word ID
    pub fn entry(&self) -> EntryId {
        EntryId::from_raw(self.raw & WORD_MASK)
    }

    /// Convert to raw representation
    pub fn as_raw(&self) -> u32 {
        self.raw
    }

    /// Check if the word comes from the system dictionary
    pub fn is_system(&self) -> bool {
        self.dict().is_system()
    }

    /// Check if the word comes from the user dictionary
    pub fn is_user(&self) -> bool {
        self.dict().is_user()
    }

    /// Check if the word is OOV
    /// An OOV node can come of OOV handlers or be a special system node like BOS or EOS
    pub fn is_oov(&self) -> bool {
        self.dict().is_oov()
    }

    /// Checks if the WordId corresponds to a special node
    pub fn is_special(&self) -> bool {
        // only beginning-of-sentence and end-of-sentence are special.
        self == &Self::BOS || self == &Self::EOS
    }

    pub const INVALID: WordId = WordId::from_raw(0xffff_ffff);
    pub const OOV_NOPOS: WordId = WordId::from_raw(0xf000_ffff);
    pub const BOS: WordId = WordId::from_raw(0xffff_fff0);
    pub const EOS: WordId = WordId::from_raw(0xffff_fff1);
}

/// Word reference
///
/// Reference which points to a entry in the system or user dictionary.
/// Similar to the WordId but the dict id part is a flag which indicates if it is a system or user word.
///
/// Top 4 bit is 0000 - points to the word in the system dictionary.
/// Top 4 bit is 0001 - points to the word in the user dictionary which this wordref is used in.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(transparent)]
pub struct WordRef {
    raw: u32,
}

impl Default for WordRef {
    fn default() -> Self {
        Self::INVALID
    }
}

impl Debug for WordRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl Display for WordRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "({}, {})",
            if self.is_system() { "sys" } else { "usr" },
            self.entry().as_raw()
        )
    }
}

impl WordRef {
    /// Create WordRef from the compressed representation
    pub const fn from_raw(raw: u32) -> WordRef {
        WordRef { raw }
    }

    /// Create a new WordRef
    pub fn new(is_system: bool, entry: u32) -> WordRef {
        debug_assert_eq!(entry & (!WORD_MASK), 0);
        let dic_part = if is_system { 0 } else { 1 << 28 };
        let word_part = entry & WORD_MASK;
        WordRef {
            raw: dic_part | word_part,
        }
    }

    /// Create a new WordRef with correctness checking
    pub fn checked(is_system: bool, entry: u32) -> SudachiResult<WordRef> {
        if entry & !WORD_MASK != 0 {
            return Err(SudachiError::LexiconSetError(
                LexiconSetError::TooLargeWordId(entry, WORD_MASK as usize),
            ));
        }
        Ok(Self::new(is_system, entry))
    }

    /// Check if the WordRef points to a system word
    pub fn is_system(&self) -> bool {
        self.raw >> 28 == 0
    }

    /// Check if the WordRef points to a user word
    pub fn is_user(&self) -> bool {
        self.raw >> 28 == 1
    }

    /// Extract Entry ID
    pub fn entry(&self) -> EntryId {
        EntryId::from_raw(self.raw & WORD_MASK)
    }

    /// Convert to raw representation
    pub fn as_raw(&self) -> u32 {
        self.raw
    }

    /// Resolve the WordRef with its DictId in the dictionary
    pub fn resolve(&self, dict: DictId) -> WordId {
        if self.is_system() {
            // dict part of system wordref is 0 and it is already resolved.
            WordId::from_raw(self.as_raw())
        } else {
            // set actual dict id for user wordref.
            WordId::from_parts(dict, self.entry())
        }
    }

    /// resolve raw
    pub fn resolve_raw(raw: u32, dict: DictId) -> u32 {
        WordRef::from_raw(raw).resolve(dict).as_raw()
    }

    pub const INVALID: WordRef = WordRef::from_raw(0xffff_ffff);
}

#[cfg(test)]
mod test {
    use super::*;

    fn assert_create(dic: u8, word: u32) {
        let id = WordId::new(dic, word);
        assert_eq!(dic, id.dict().as_raw());
        assert_eq!(word, id.entry().as_raw());
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

    #[test]
    fn is_special() {
        assert!(WordId::EOS.is_special());
        assert!(WordId::BOS.is_special());
        assert!(!WordId::INVALID.is_special());
        assert!(!WordId::new(0, 0).is_special());
    }
}
