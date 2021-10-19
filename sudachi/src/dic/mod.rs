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

use nom::number::complete::{le_u16, le_u32, le_u8};
use nom::Parser;
use std::path::PathBuf;

use crate::dic::word_id::WordId;
use crate::error::{SudachiNomError, SudachiNomResult};
use crate::prelude::*;
use character_category::CharacterCategory;
use grammar::Grammar;
use header::Header;
use lexicon::Lexicon;
use lexicon_set::LexiconSet;

pub mod category_type;
pub mod character_category;
pub mod connect;
pub mod dictionary;
pub mod grammar;
pub mod header;
pub mod lexicon;
pub mod lexicon_set;
pub mod word_id;

/// A dictionary consists of one system_dict and zero or more user_dicts
pub struct LoadedDictionary<'a> {
    pub grammar: Grammar<'a>,
    pub lexicon_set: LexiconSet<'a>,
}

impl<'a> LoadedDictionary<'a> {
    /// Creates a system dictionary from bytes, and load a character category from file
    pub fn from_system_dictionary(
        dictionary_bytes: &'a [u8],
        character_category_file: &PathBuf,
    ) -> SudachiResult<LoadedDictionary<'a>> {
        let system_dict = DictionaryLoader::read_system_dictionary(dictionary_bytes)?;

        let character_category = CharacterCategory::from_file(character_category_file)?;
        let mut grammar = system_dict
            .grammar
            .ok_or(SudachiError::InvalidDictionaryGrammar)?;
        grammar.set_character_category(character_category);

        Ok(LoadedDictionary {
            grammar,
            lexicon_set: LexiconSet::new(system_dict.lexicon),
        })
    }
}

/// A single system or user dictionary
pub struct DictionaryLoader<'a> {
    pub header: Header,
    pub grammar: Option<Grammar<'a>>,
    pub lexicon: Lexicon<'a>,
}

impl<'a> DictionaryLoader<'a> {
    /// Creates a binary dictionary from bytes
    fn read_dictionary(dictionary_bytes: &[u8]) -> SudachiResult<DictionaryLoader> {
        let header = Header::new(&dictionary_bytes[..Header::STORAGE_SIZE])?;
        let mut offset = Header::STORAGE_SIZE;

        let grammar = if header.has_grammar() {
            let tmp = Grammar::new(dictionary_bytes, offset)?;
            offset += tmp.storage_size;
            Some(tmp)
        } else {
            None
        };

        let lexicon = Lexicon::new(dictionary_bytes, offset, header.has_synonym_group_ids())?;

        Ok(DictionaryLoader {
            header,
            grammar,
            lexicon,
        })
    }

    /// Creates a system binary dictionary from bytes
    ///
    /// Returns Err if header version is not match
    pub fn read_system_dictionary(dictionary_bytes: &[u8]) -> SudachiResult<DictionaryLoader> {
        let dict = Self::read_dictionary(dictionary_bytes)?;
        match dict.header.version {
            header::HeaderVersion::SystemDict(_) => Ok(dict),
            _ => Err(SudachiError::InvalidHeader(
                header::HeaderError::InvalidSystemDictVersion,
            )),
        }
    }

    /// Creates a user binary dictionary from bytes
    ///
    /// Returns Err if header version is not match
    pub fn read_user_dictionary(dictionary_bytes: &[u8]) -> SudachiResult<DictionaryLoader> {
        let dict = Self::read_dictionary(dictionary_bytes)?;
        match dict.header.version {
            header::HeaderVersion::UserDict(_) => Ok(dict),
            _ => Err(SudachiError::InvalidHeader(
                header::HeaderError::InvalidSystemDictVersion,
            )),
        }
    }
}

fn u32_array_parser(input: &[u8]) -> SudachiNomResult<&[u8], Vec<u32>> {
    let (rest, length) = le_u8(input)?;
    nom::multi::count(le_u32, length as usize)(rest)
}

fn u32_wid_array_parser(input: &[u8]) -> SudachiNomResult<&[u8], Vec<WordId>> {
    let (rest, length) = le_u8(input)?;
    nom::multi::count(le_u32.map(|id| WordId::from_raw(id)), length as usize)(rest)
}

fn utf16_string_parser(input: &[u8]) -> SudachiNomResult<&[u8], String> {
    let (rest, length) = string_length_parser(input)?;
    let (rest, vs) = nom::multi::count(le_u16, length as usize)(rest)?;
    Ok((
        rest,
        String::from_utf16(&vs).map_err(|_| nom::Err::Failure(SudachiNomError::Utf16String))?,
    ))
}

fn string_length_parser(input: &[u8]) -> SudachiNomResult<&[u8], u16> {
    let (rest, length) = le_u8(input)?;
    // word length can be 1 or 2 bytes
    let (rest, opt_low) = nom::combinator::cond(length >= 128, le_u8)(rest)?;
    Ok((
        rest,
        match opt_low {
            Some(low) => ((length as u16 & 0x7F) << 8) | low as u16,
            None => length as u16,
        },
    ))
}
