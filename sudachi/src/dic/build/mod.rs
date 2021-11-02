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

use std::io::Write;
use std::path::Path;

use crate::analysis::stateless_tokenizer::DictionaryAccess;
use crate::dic::build::error::{DicCompilationCtx, DicWriteError, DicWriteReason};
use crate::dic::build::index::IndexBuilder;
use crate::dic::build::lexicon::LexiconWriter;
use crate::dic::build::resolve::{BuiltDictResolver, ChainedResolver, RawDictResolver};
use crate::dic::dictionary::JapaneseDictionary;
use crate::dic::header::{Header, HeaderVersion, SystemDictVersion, UserDictVersion};
use crate::dic::word_id::WordId;
use crate::error::SudachiResult;

pub(crate) mod conn;
pub mod error;
pub(crate) mod index;
pub(crate) mod lexicon;
pub(crate) mod parse;
pub(crate) mod primitives;
mod resolve;
#[cfg(test)]
mod test;

const MAX_POS_IDS: usize = i16::MAX as usize;
const MAX_DIC_STRING_LEN: usize = MAX_POS_IDS;
const MAX_ARRAY_LEN: usize = i8::MAX as usize;

pub enum DataSource<'a> {
    File(&'a Path),
    Data(&'a [u8]),
}

pub trait AsDataSource<'a> {
    fn convert(self) -> DataSource<'a>;
}

impl<'a> AsDataSource<'a> for &'a Path {
    fn convert(self) -> DataSource<'a> {
        DataSource::File(self)
    }
}

impl<'a> AsDataSource<'a> for &'a [u8] {
    fn convert(self) -> DataSource<'a> {
        DataSource::Data(self)
    }
}

impl<'a, const N: usize> AsDataSource<'a> for &'a [u8; N] {
    fn convert(self) -> DataSource<'a> {
        DataSource::Data(&self[..])
    }
}

pub struct DictBuilder {
    user: bool,
    lexicon: lexicon::LexiconReader,
    conn: conn::ConnBuffer,
    ctx: DicCompilationCtx,
    header: Header,
    resolved: bool,
}

impl DictBuilder {
    pub fn new() -> Self {
        DictBuilder {
            user: false,
            lexicon: lexicon::LexiconReader::new(),
            conn: conn::ConnBuffer::new(),
            ctx: DicCompilationCtx::default(),
            header: Header::new(),
            resolved: false,
        }
    }

    pub fn set_user(&mut self, user: bool) {
        if user {
            self.header.version = HeaderVersion::UserDict(UserDictVersion::Version3)
        } else {
            self.header.version = HeaderVersion::SystemDict(SystemDictVersion::Version2)
        }
        self.user = user;
    }

    pub fn set_description(&mut self, description: String) {
        self.header.description = description
    }

    pub fn read_lexicon<'a, T: AsDataSource<'a> + 'a>(&mut self, data: T) -> SudachiResult<usize> {
        match data.convert() {
            DataSource::File(p) => self.lexicon.read_file(p),
            DataSource::Data(d) => self.lexicon.read_bytes(d),
        }
    }

    pub fn read_conn<'a, T: AsDataSource<'a> + 'a>(&mut self, data: T) -> SudachiResult<()> {
        match data.convert() {
            DataSource::File(p) => self.conn.read_file(p),
            DataSource::Data(d) => self.conn.read(d),
        }
    }

    pub fn compile<W: Write>(&mut self, w: &mut W) -> SudachiResult<()> {
        self.check_if_resolved()?;
        let mut written = self.header.write_to(w)?;
        written += self.write_grammar(w)?;
        self.write_lexicon(w, written)?;
        Ok(())
    }

    fn write_grammar<W: Write>(&mut self, w: &mut W) -> SudachiResult<usize> {
        let mut size = 0;
        size += self.lexicon.write_pos_table(w)?;
        size += self.conn.write_to(w)?;
        Ok(size)
    }

    fn write_index<W: Write>(&mut self, w: &mut W) -> SudachiResult<usize> {
        let mut size = 0;
        let mut index = IndexBuilder::new();
        for (i, e) in self.lexicon.entries().iter().enumerate() {
            if e.should_index() {
                let wid = WordId::checked(0, i as u32)?;
                index.add(e.surface(), wid);
            }
        }

        let word_id_table = index.build_word_id_table()?;
        let trie = index.build_trie()?;

        let trie_size = trie.len() / 4;
        w.write_all(&(trie_size as u32).to_le_bytes())?;
        size += 4;
        w.write_all(&trie)?;
        size += trie.len();
        std::mem::drop(trie); //can be big, so drop explicitly

        w.write_all(&(word_id_table.len() as u32).to_le_bytes())?;
        size += 4;
        w.write_all(&word_id_table)?;
        size += word_id_table.len();

        Ok(size)
    }

    fn write_lexicon<W: Write>(&mut self, w: &mut W, offset: usize) -> SudachiResult<usize> {
        let mut size = self.write_index(w)?;
        let mut writer = LexiconWriter::new(self.lexicon.entries(), offset + size);
        size += writer.write(w)?;
        Ok(size)
    }

    fn check_if_resolved(&self) -> SudachiResult<()> {
        if self.lexicon.needs_split_resolution() && !self.resolved {
            return self.ctx.err(DicWriteReason::UnresolvedSplits);
        }

        Ok(())
    }

    /// this function must only be used in resolve_splits
    fn unsafe_make_resolver<'a, 'b>(&'a self) -> RawDictResolver<'b> {
        let resolver = RawDictResolver::new(self.lexicon.entries(), self.user);
        // resolver borrows parts of entries, but it does not touch splits
        // resolve function only modifies splits
        unsafe { std::mem::transmute(resolver) }
    }

    pub fn resolve(&mut self) -> SudachiResult<usize> {
        self.resolve_dict::<&JapaneseDictionary>(None)
    }

    pub fn resolve_dict<D: DictionaryAccess>(&mut self, system: Option<D>) -> SudachiResult<usize> {
        if !self.lexicon.needs_split_resolution() {
            self.resolved = true;
            return Ok(0);
        }

        let this_resolver = self.unsafe_make_resolver();

        let cnt = match system {
            Some(d) => {
                let built_resolver = BuiltDictResolver::new(d);
                let chained = ChainedResolver::new(this_resolver, built_resolver);
                self.lexicon.resolve_splits(&chained)
            }
            None => self.lexicon.resolve_splits(&this_resolver),
        };
        match cnt {
            Ok(cnt) => {
                self.resolved = true;
                Ok(cnt)
            }
            Err((split_info, line)) => Err(DicWriteError {
                file: "<entries>".to_owned(),
                line,
                cause: DicWriteReason::InvalidSplitWordReference(split_info),
            }
            .into()),
        }
    }
}
