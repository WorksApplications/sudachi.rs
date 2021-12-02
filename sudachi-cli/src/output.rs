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

use std::io::{BufWriter, Write};
use sudachi::analysis::morpheme::Morpheme;
use sudachi::analysis::stateless_tokenizer::DictionaryAccess;
use sudachi::dic::subset::InfoSubset;

use sudachi::prelude::{MorphemeList, SudachiResult};

pub type Writer = BufWriter<Box<dyn Write>>;

pub trait SudachiOutput<T> {
    fn write(&self, writer: &mut Writer, morphemes: &MorphemeList<T>) -> SudachiResult<()>;
    fn subset(&self) -> InfoSubset;
}

pub struct Wakachi {
    word_separator: String,
    sentence_separator: String,
}

impl Wakachi {
    pub fn default() -> Wakachi {
        Wakachi {
            word_separator: String::from(" "),
            sentence_separator: String::from("\n"),
        }
    }
}

impl<T: DictionaryAccess> SudachiOutput<T> for Wakachi {
    fn write(&self, writer: &mut Writer, morphemes: &MorphemeList<T>) -> SudachiResult<()> {
        if morphemes.len() == 0 {
            writer.write_all(b"\n")?;
            return Ok(());
        }
        let last_idx = morphemes.len() - 1;
        for m in morphemes.iter() {
            writer.write_all(m.surface().as_bytes())?;
            let trailer = if m.index() != last_idx {
                &self.word_separator
            } else {
                &self.sentence_separator
            };
            writer.write_all(trailer.as_bytes())?;
        }
        Ok(())
    }

    fn subset(&self) -> InfoSubset {
        InfoSubset::empty()
    }
}

pub struct Simple {
    print_all: bool,
}

impl Simple {
    pub fn new(print_all: bool) -> Simple {
        Simple { print_all }
    }
}

impl<T: DictionaryAccess> SudachiOutput<T> for Simple {
    fn write(&self, writer: &mut Writer, morphemes: &MorphemeList<T>) -> SudachiResult<()> {
        for m in morphemes.iter() {
            write_morpheme_basic(writer, &m)?;
            if self.print_all {
                write_morpheme_extended(writer, &m)?
            }
            writer.write_all(b"\n")?;
        }
        writer.write_all(b"EOS\n")?;
        Ok(())
    }

    fn subset(&self) -> InfoSubset {
        let mut subset = InfoSubset::POS_ID | InfoSubset::NORMALIZED_FORM;

        if self.print_all {
            subset |= InfoSubset::DIC_FORM_WORD_ID
                | InfoSubset::READING_FORM
                | InfoSubset::SYNONYM_GROUP_ID;
        }

        subset
    }
}

#[inline]
fn write_morpheme_basic<T: DictionaryAccess>(
    writer: &mut Writer,
    morpheme: &Morpheme<T>,
) -> SudachiResult<()> {
    writer.write_all(morpheme.surface().as_bytes())?;
    writer.write_all(b"\t")?;
    let all_pos = morpheme.part_of_speech();
    for (idx, pos) in all_pos.iter().enumerate() {
        writer.write_all(pos.as_bytes())?;
        if idx + 1 != all_pos.len() {
            writer.write_all(b",")?;
        }
    }
    writer.write_all(b"\t")?;
    writer.write_all(morpheme.normalized_form().as_bytes())?;
    Ok(())
}

#[inline]
fn write_morpheme_extended<T: DictionaryAccess>(
    writer: &mut Writer,
    morpheme: &Morpheme<T>,
) -> SudachiResult<()> {
    write!(
        writer,
        "\t{}\t{}\t{}\t{:?}",
        morpheme.dictionary_form(),
        morpheme.reading_form(),
        morpheme.dictionary_id(),
        morpheme.synonym_group_ids(),
    )?;
    if morpheme.is_oov() {
        writer.write_all(b"\t(OOV)")?;
    }
    Ok(())
}
