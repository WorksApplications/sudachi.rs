/*
 * Copyright (c) 2026 Works Applications Co., Ltd.
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

use crate::dic::lexicon_set::LexiconSetError;
use crate::dic::read::utf16_string::utf16_string_of_length;
use crate::error::{SudachiError, SudachiResult};

pub struct CompactedStrings<'a> {
    bytes: &'a [u8],
}

impl<'a> CompactedStrings<'a> {
    pub fn from_bytes(bytes: &'a [u8]) -> CompactedStrings<'a> {
        CompactedStrings { bytes }
    }

    pub fn get_string(&self, pointer: StringPointer) -> SudachiResult<String> {
        let (_, parsed) = utf16_string_of_length(
            &self.bytes[(pointer.offset as usize * 2)..],
            pointer.length as usize,
        )?;
        Ok(parsed)
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct StringPointer {
    /// length of the string (in utf16 codepoint)
    pub length: u32,
    /// offset in the CompactedStrings (in utf16 codepoint)
    pub offset: u32,
}

impl StringPointer {
    /// bit count of the base part
    pub const BASE_LENGTH_BITS: u32 = 5;
    /// offset to the base part
    pub const BASE_LENGTH_OFFSET: u32 = 32 - Self::BASE_LENGTH_BITS;
    /// max bit count of the additional length part (in value)
    /// note that its top 1 bit is not stored in byte representation.
    pub const MAX_VARIABLE_LENGTH_BITS: u32 = 12;
    /// max string length that can be stored using base part only
    pub const MAX_SIMPLE_LENGTH: u32 =
        2u32.pow(Self::BASE_LENGTH_BITS) - 1 - Self::MAX_VARIABLE_LENGTH_BITS;
    /// max string length that can be stored
    pub const MAX_LENGTH: u32 =
        2u32.pow(Self::MAX_VARIABLE_LENGTH_BITS) - 1 + Self::MAX_SIMPLE_LENGTH;

    /// Check if the given length and offset satisfy the constraints for a valid StringPointer.
    fn is_valid(length: u32, offset: u32) -> bool {
        if length > Self::MAX_LENGTH {
            return false;
        }
        let alignment = Self::required_alignment(length);
        if alignment == 0 {
            return true;
        }
        let alignment_mask = (1 << (alignment - 1)) - 1;
        (offset & alignment_mask) == 0
    }

    /// Calculate the required alignment bits for the offset from the length.
    fn required_alignment(length: u32) -> u32 {
        if length <= Self::MAX_SIMPLE_LENGTH {
            0
        } else {
            let remaining = length - Self::MAX_SIMPLE_LENGTH;
            32 - remaining.leading_zeros()
        }
    }

    pub fn unchecked(length: u32, offset: u32) -> Self {
        StringPointer { length, offset }
    }

    pub fn checked(length: u32, offset: u32) -> SudachiResult<Self> {
        if !Self::is_valid(length, offset) {
            return Err(SudachiError::LexiconSetError(
                LexiconSetError::InvalidStringPointer(
                    length as usize,
                    offset as usize,
                    Self::required_alignment(length) as usize,
                ),
            ));
        }
        Ok(Self::unchecked(length, offset))
    }

    pub fn encode(&self) -> u32 {
        let additional_length_bits = Self::required_alignment(self.length);
        let base_length = std::cmp::min(self.length, Self::MAX_SIMPLE_LENGTH);
        let base_part = (base_length + additional_length_bits) << Self::BASE_LENGTH_OFFSET;

        let additional_length = self.length - base_length;
        let implicit_bit = (1 << Self::MAX_VARIABLE_LENGTH_BITS) >> (13 - additional_length_bits);
        let non_fixed_length = additional_length ^ implicit_bit;
        let variable_part =
            non_fixed_length << (16 + Self::MAX_VARIABLE_LENGTH_BITS - additional_length_bits);

        let offset_part = self.offset >> additional_length_bits.saturating_sub(1);

        debug_assert!(base_part & variable_part == 0);
        debug_assert!(base_part & offset_part == 0);
        debug_assert!(variable_part & offset_part == 0);
        base_part | variable_part | offset_part
    }

    pub fn decode(encoded: u32) -> Self {
        // first 5 bits are length and marker values for additional length bits
        let base_value = encoded >> Self::BASE_LENGTH_OFFSET;
        let additional_length_bits = base_value.saturating_sub(Self::MAX_SIMPLE_LENGTH);
        // additional length bits are stored in the following
        let non_fixed_length = (encoded & 0x07ff_0000)
            >> (16 + Self::MAX_VARIABLE_LENGTH_BITS - additional_length_bits);
        // compute the non-stored first bit which is implicitly one
        let implicit_bit = (1 << Self::MAX_VARIABLE_LENGTH_BITS) >> (13 - additional_length_bits);
        let length = (base_value - additional_length_bits) + (non_fixed_length | implicit_bit);
        // offset are aligned based on the additional length bits
        let alignment = additional_length_bits.saturating_sub(1);
        let offset = (encoded & (0x07ff_ffff >> alignment)) << alignment;

        StringPointer { length, offset }
    }
}
