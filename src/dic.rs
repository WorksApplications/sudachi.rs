use nom::{le_u16, le_u8};
use std::path::PathBuf;

use crate::error::SudachiNomCustomError;
use crate::prelude::*;

pub mod category_type;
pub mod character_category;
pub mod grammar;
pub mod header;
pub mod lexicon;
pub mod lexicon_set;

use character_category::CharacterCategory;
use grammar::Grammar;
use header::Header;
use lexicon::Lexicon;
use lexicon_set::LexiconSet;

/// A dictionary consists of one system_dict and zero or more user_dicts
pub struct Dictionary<'a> {
    pub grammar: Grammar<'a>,
    pub lexicon_set: LexiconSet<'a>,
}

impl<'a> Dictionary<'a> {
    /// Creates a system dictionary from bytes, and load a character category from file
    pub fn from_system_dictionary(
        dictionary_bytes: &'a [u8],
        character_category_file: PathBuf,
    ) -> SudachiResult<Dictionary<'a>> {
        let system_dict = BinaryDictionary::from_system_dictionary(dictionary_bytes)?;

        let character_category = CharacterCategory::from_file(character_category_file)?;
        let mut grammar = system_dict
            .grammar
            .ok_or(SudachiError::InvalidDictionaryGrammar)?;
        grammar.set_character_category(character_category);

        Ok(Dictionary {
            grammar,
            lexicon_set: LexiconSet::new(system_dict.lexicon),
        })
    }
}

/// A single system or user dictionary
pub struct BinaryDictionary<'a> {
    pub header: Header,
    pub grammar: Option<Grammar<'a>>,
    pub lexicon: Lexicon<'a>,
}

impl<'a> BinaryDictionary<'a> {
    /// Creates a binary dictionary from bytes
    fn read_dictionary(dictionary_bytes: &[u8]) -> SudachiResult<BinaryDictionary> {
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

        Ok(BinaryDictionary {
            header,
            grammar,
            lexicon,
        })
    }

    /// Creates a system binary dictionary from bytes
    ///
    /// Returns Err if header version is not match
    pub fn from_system_dictionary(dictionary_bytes: &[u8]) -> SudachiResult<BinaryDictionary> {
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
    pub fn from_user_dictionary(dictionary_bytes: &[u8]) -> SudachiResult<BinaryDictionary> {
        let dict = Self::read_dictionary(dictionary_bytes)?;
        match dict.header.version {
            header::HeaderVersion::UserDict(_) => Ok(dict),
            _ => Err(SudachiError::InvalidHeader(
                header::HeaderError::InvalidSystemDictVersion,
            )),
        }
    }
}

named!(
    utf16_string<&[u8], String>,
    do_parse!(
        length: le_u8 >>
        v: count!(le_u16, length as usize) >>

        (String::from_utf16(&v)
            .map_err(|_| nom::Err::Failure(
                nom::Context::Code(&[] as &[u8], nom::ErrorKind::Custom(SudachiNomCustomError::FromUtf16Nom as u32))))?
        )
    )
);
