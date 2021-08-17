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
    pub fn from_system_dicrionary(
        dictionary_bytes: &'a [u8],
        character_category_file: Option<PathBuf>,
    ) -> SudachiResult<Dictionary<'a>> {
        let system_dict = BinaryDictionary::from_system_dicrionary(dictionary_bytes)?;

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

    pub fn merge_user_dictionary(
        &mut self,
        dictionary_bytes: &'a [u8],
        tokenizer: Tokenizer,
    ) -> SudachiResult<()> {
        let user_dict = BinaryDictionary::from_user_dicrionary(dictionary_bytes)?;

        // we need to update lexicon first, since it needs the current number of pos
        let mut user_lexicon = user_dict.lexicon;
        user_lexicon.update_cost(tokenizer)?;
        self.lexicon_set
            .append(user_lexicon, self.grammar.pos_list.len())?;

        if let Some(g) = user_dict.grammar {
            self.grammar.merge(&g);
        }

        Ok(())
    }
}

/// A single system or user dictionary
pub struct BinaryDictionary<'a> {
    pub header: Header,
    pub grammar: Option<Grammar<'a>>,
    pub lexicon: Lexicon<'a>,
}

impl<'a> BinaryDictionary<'a> {
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

        let lexicon = Lexicon::new(dictionary_bytes, offset)?;

        Ok(BinaryDictionary {
            header,
            grammar,
            lexicon,
        })
    }
    pub fn from_system_dicrionary(dictionary_bytes: &[u8]) -> SudachiResult<BinaryDictionary> {
        let dict = Self::read_dictionary(dictionary_bytes)?;
        match dict.header.version {
            header::HeaderVersion::SystemDict(_) => Ok(dict),
            _ => Err(SudachiError::InvalidHeader(
                header::HeaderError::InvalidSystemDictVersion,
            )),
        }
    }
    pub fn from_user_dicrionary(dictionary_bytes: &[u8]) -> SudachiResult<BinaryDictionary> {
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
