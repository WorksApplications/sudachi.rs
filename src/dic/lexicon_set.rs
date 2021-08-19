use thiserror::Error;

use crate::dic::lexicon::word_infos::WordInfo;
use crate::dic::lexicon::Lexicon;
use crate::prelude::*;

/// Sudachi error
#[derive(Error, Debug, Eq, PartialEq)]
pub enum LexiconSetError {
    #[error("too large word_id {0} in dict {1}")]
    TooLargeWordId(u32, usize),

    #[error("too large dictionary_id {0}")]
    TooLargeDictionaryId(usize),

    #[error("too many user dictionaries")]
    TooManyDictionaries,
}

pub struct LexiconSet<'a> {
    lexicons: Vec<Lexicon<'a>>,
    pos_offsets: Vec<usize>,
}

impl<'a> LexiconSet<'a> {
    const MAX_DICTIONARIES: usize = 16;

    pub fn new(system_lexicon: Lexicon) -> LexiconSet {
        LexiconSet {
            lexicons: vec![system_lexicon],
            pos_offsets: vec![0],
        }
    }

    pub fn append(
        &mut self,
        lexicon: Lexicon<'a>,
        pos_offset: usize,
    ) -> Result<(), LexiconSetError> {
        if self.is_full() {
            return Err(LexiconSetError::TooManyDictionaries);
        }

        self.lexicons.push(lexicon);
        self.pos_offsets.push(pos_offset);
        Ok(())
    }

    pub fn is_full(&self) -> bool {
        self.lexicons.len() >= LexiconSet::MAX_DICTIONARIES
    }
}

impl LexiconSet<'_> {
    pub fn lookup(&self, input: &[u8], offset: usize) -> SudachiResult<Vec<(u32, usize)>> {
        let mut vs: Vec<(u32, usize)> = Vec::new();
        for (did, user_lexicon) in self.lexicons.iter().enumerate().skip(1) {
            // lookup user dictionary first
            // todo: impl Iterator
            vs.extend(
                user_lexicon
                    .lookup(input, offset)?
                    .iter()
                    .map(|(wid, l)| self.build_dictword_id(did, *wid).map(|dwid| (dwid, *l)))
                    .collect::<Result<Vec<_>, _>>()?,
            );
        }
        vs.extend(self.lexicons[0].lookup(input, offset)?);

        Ok(vs)
    }

    pub fn get_word_info(&self, dictword_id: u32) -> SudachiResult<WordInfo> {
        let (dict_id, word_id) = LexiconSet::decode_dictword_id(dictword_id);
        let mut word_info = self.lexicons[dict_id].get_word_info(word_id)?;
        let pos_id = word_info.pos_id;
        if dict_id > 0 && pos_id as usize >= self.pos_offsets[1] {
            // user defined part-of-speech
            // todo: this may overflow
            word_info.pos_id =
                (pos_id as usize - self.pos_offsets[1] + self.pos_offsets[dict_id]) as u16;
        }
        self.update_dict_id(&mut word_info.a_unit_split, dict_id)?;
        self.update_dict_id(&mut word_info.b_unit_split, dict_id)?;
        self.update_dict_id(&mut word_info.word_structure, dict_id)?;

        Ok(word_info)
    }

    pub fn get_word_param(&self, dictword_id: u32) -> SudachiResult<(i16, i16, i16)> {
        let (dict_id, word_id) = LexiconSet::decode_dictword_id(dictword_id);
        self.lexicons[dict_id].get_word_param(word_id)
    }

    /// Merge dict_id and word_id into one u32
    ///
    /// We use top 4 bits for dict_id
    fn build_dictword_id(&self, dict_id: usize, word_id: u32) -> Result<u32, LexiconSetError> {
        if word_id > 0x0FFFFFFF {
            return Err(LexiconSetError::TooLargeWordId(word_id, dict_id));
        }
        if dict_id > self.lexicons.len() {
            return Err(LexiconSetError::TooLargeDictionaryId(dict_id));
        }
        Ok((dict_id as u32) << 28 | word_id)
    }
    pub fn get_dictionary_id(dictword_id: u32) -> usize {
        (dictword_id >> 28) as usize
    }
    fn get_word_id(dictword_id: u32) -> u32 {
        dictword_id & 0x0FFFFFFF
    }
    fn decode_dictword_id(dictword_id: u32) -> (usize, u32) {
        let dict_id = LexiconSet::get_dictionary_id(dictword_id);
        let word_id = LexiconSet::get_word_id(dictword_id);
        (dict_id, word_id)
    }

    fn update_dict_id(&self, split: &mut Vec<u32>, dict_id: usize) -> SudachiResult<()> {
        for i in 0..split.len() {
            let (crr_dict_id, word_id) = LexiconSet::decode_dictword_id(split[i]);
            if crr_dict_id > 0 {
                // update if target word is not in system_dict
                split[i] = self.build_dictword_id(dict_id, word_id)?;
            }
        }
        Ok(())
    }

    pub fn size(&self) -> u32 {
        self.lexicons.iter().fold(0, |acc, lex| acc + lex.size())
    }
}
