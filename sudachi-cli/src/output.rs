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

use sudachi::prelude::{MorphemeList, SudachiResult};

type Writer = BufWriter<Box<dyn Write>>;

pub trait SudachiOutput<T> {
    fn write(&self, writer: &mut Writer, morphemes: &MorphemeList<T>) -> SudachiResult<()>;
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
    // this implementation uses raw writes which can truncate output
    // if the thing to write is larger than
    // the buffer of BufWrite (8k), our inputs are 100% lesser than that
    fn write(&self, writer: &mut Writer, morphemes: &MorphemeList<T>) -> SudachiResult<()> {
        if morphemes.len() == 0 {
            writer.write(b"\n")?;
            return Ok(());
        }
        let last_idx = morphemes.len() - 1;
        for m in morphemes.iter() {
            writer.write(m.surface().as_bytes())?;
            let trailer = if m.index() != last_idx {
                &self.word_separator
            } else {
                &self.sentence_separator
            };
            writer.write(trailer.as_bytes())?;
        }
        Ok(())
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
    // this implementation uses raw writes which can truncate output
    // if the thing to write is larger than
    // the buffer of BufWrite (8k), our inputs are 100% lesser than that
    fn write(&self, writer: &mut Writer, morphemes: &MorphemeList<T>) -> SudachiResult<()> {
        for m in morphemes.iter() {
            write_morpheme_basic(writer, &m)?;
            if self.print_all {
                write_morpheme_extended(writer, &m)?
            }
            writer.write(b"\n")?;
        }
        writer.write(b"EOS\n")?;
        Ok(())
    }
}

// this implementation uses raw writes which can truncate output
// if the thing to write is larger than
// the buffer of BufWrite (8k), our inputs are 100% lesser than that
#[inline]
fn write_morpheme_basic<T: DictionaryAccess>(
    writer: &mut Writer,
    morpheme: &Morpheme<T>,
) -> SudachiResult<()> {
    writer.write(morpheme.surface().as_bytes())?;
    writer.write(b"\t")?;
    let all_pos = morpheme.part_of_speech()?;
    for (idx, pos) in all_pos.iter().enumerate() {
        writer.write(pos.as_bytes())?;
        if idx + 1 != all_pos.len() {
            writer.write(b",")?;
        }
    }
    writer.write(b"\t")?;
    writer.write(morpheme.normalized_form().as_bytes())?;
    Ok(())
}

// this implementation uses raw writes which can truncate output
// if the thing to write is larger than
// the buffer of BufWrite (8k), our inputs are 100% lesser than that
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
        writer.write(b"\t(OOV)")?;
    }
    Ok(())
}
