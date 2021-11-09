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
use crate::dic::build::error::{BuildFailure, DicBuildError, DicCompilationCtx};
use crate::dic::build::index::IndexBuilder;
use crate::dic::build::lexicon::LexiconWriter;
use crate::dic::build::report::{DictPartReport, ReportBuilder, Reporter};
use crate::dic::build::resolve::{BinDictResolver, ChainedResolver, RawDictResolver};
use crate::dic::grammar::Grammar;
use crate::dic::header::{Header, HeaderVersion, SystemDictVersion, UserDictVersion};
use crate::dic::lexicon_set::LexiconSet;
use crate::dic::word_id::WordId;
use crate::error::SudachiResult;
use crate::plugin::input_text::InputTextPlugin;
use crate::plugin::oov::OovProviderPlugin;
use crate::plugin::path_rewrite::PathRewritePlugin;

pub(crate) mod conn;
pub mod error;
pub(crate) mod index;
pub(crate) mod lexicon;
pub(crate) mod parse;
pub(crate) mod primitives;
pub mod report;
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
    fn name(&self) -> String;
}

impl<'a> AsDataSource<'a> for DataSource<'a> {
    fn convert(self) -> DataSource<'a> {
        self
    }

    fn name(&self) -> String {
        match self {
            DataSource::File(p) => p.to_str().map(|s| s.to_owned()).unwrap_or_default(),
            DataSource::Data(d) => format!("memory ({} bytes)", d.len()),
        }
    }
}

impl<'a> AsDataSource<'a> for &'a Path {
    fn convert(self) -> DataSource<'a> {
        DataSource::File(self)
    }
    fn name(&self) -> String {
        self.to_str().map(|s| s.to_owned()).unwrap_or_default()
    }
}

impl<'a> AsDataSource<'a> for &'a [u8] {
    fn convert(self) -> DataSource<'a> {
        DataSource::Data(self)
    }
    fn name(&self) -> String {
        format!("memory ({} bytes)", self.len())
    }
}

impl<'a, const N: usize> AsDataSource<'a> for &'a [u8; N] {
    fn convert(self) -> DataSource<'a> {
        DataSource::Data(&self[..])
    }
    fn name(&self) -> String {
        format!("memory ({} bytes)", self.len())
    }
}

pub enum NoDic {}

impl DictionaryAccess for NoDic {
    fn grammar(&self) -> &Grammar<'_> {
        panic!("there is no grammar here")
    }

    fn lexicon(&self) -> &LexiconSet<'_> {
        panic!("there is no lexicon here")
    }

    fn input_text_plugins(&self) -> &[Box<dyn InputTextPlugin + Sync + Send>] {
        return &[];
    }

    fn oov_provider_plugins(&self) -> &[Box<dyn OovProviderPlugin + Sync + Send>] {
        return &[];
    }

    fn path_rewrite_plugins(&self) -> &[Box<dyn PathRewritePlugin + Sync + Send>] {
        return &[];
    }
}

/// Builds a binary dictionary from csv lexicon and connection matrix (optional)
pub struct DictBuilder<D> {
    user: bool,
    lexicon: lexicon::LexiconReader,
    conn: conn::ConnBuffer,
    ctx: DicCompilationCtx,
    header: Header,
    resolved: bool,
    prebuilt: Option<D>,
    reporter: Reporter,
}

impl DictBuilder<NoDic> {
    /// Creates a new builder for system dictionary
    pub fn new_system() -> Self {
        Self::new_empty()
    }
}

impl<D: DictionaryAccess> DictBuilder<D> {
    fn new_empty() -> Self {
        Self {
            user: false,
            lexicon: lexicon::LexiconReader::new(),
            conn: conn::ConnBuffer::new(),
            ctx: DicCompilationCtx::default(),
            header: Header::new(),
            resolved: false,
            prebuilt: None,
            reporter: Reporter::new(),
        }
    }

    /// Creates a new builder for user dictionary
    pub fn new_user(system: D) -> Self {
        let mut bldr = Self::new_empty();
        bldr.set_user(true);
        bldr.lexicon.preload_pos(system.grammar());
        let cm = system.grammar().conn_matrix();
        bldr.lexicon
            .set_max_conn_sizes(cm.num_left() as _, cm.num_right() as _);
        bldr.lexicon
            .set_num_system_words(system.lexicon().size() as usize);
        bldr.prebuilt = Some(system);
        bldr
    }

    /// Set the dictionary compile time to the specified time
    /// instead of current time
    pub fn set_compile_time<T: Into<std::time::SystemTime>>(
        &mut self,
        time: T,
    ) -> std::time::SystemTime {
        self.header.set_time(time.into())
    }

    /// Set the dictionary description
    pub fn set_description<T: Into<String>>(&mut self, description: T) {
        self.header.description = description.into()
    }

    /// Read the csv lexicon from either a file or an in-memory buffer
    pub fn read_lexicon<'a, T: AsDataSource<'a> + 'a>(&mut self, data: T) -> SudachiResult<usize> {
        let report = ReportBuilder::new(data.name()).read();
        let result = match data.convert() {
            DataSource::File(p) => self.lexicon.read_file(p),
            DataSource::Data(d) => self.lexicon.read_bytes(d),
        };
        self.reporter.collect_r(result, report)
    }

    /// Read the connection matrix from either a file or an in-memory buffer
    pub fn read_conn<'a, T: AsDataSource<'a> + 'a>(&mut self, data: T) -> SudachiResult<()> {
        let report = ReportBuilder::new(data.name()).read();
        match data.convert() {
            DataSource::File(p) => self.conn.read_file(p),
            DataSource::Data(d) => self.conn.read(d),
        }?;
        self.lexicon
            .set_max_conn_sizes(self.conn.left(), self.conn.right());
        self.reporter.collect(
            self.conn.left() as usize * self.conn.right() as usize,
            report,
        );
        Ok(())
    }

    /// Compile the binary dictionary and write it to the specified sink
    pub fn compile<W: Write>(&mut self, w: &mut W) -> SudachiResult<()> {
        self.check_if_resolved()?;
        let report = ReportBuilder::new("validate").read();
        self.lexicon.validate_entries()?;
        self.reporter.collect(self.lexicon.entries().len(), report);
        let mut written = self.header.write_to(w)?;
        written += self.write_grammar(w)?;
        self.write_lexicon(w, written)?;
        Ok(())
    }

    /// Resolve the dictionary references.
    ///
    /// Returns the number of resolved entries
    pub fn resolve(&mut self) -> SudachiResult<usize> {
        self.resolve_impl()
    }

    /// Return dictionary build report
    pub fn report(&self) -> &[DictPartReport] {
        self.reporter.reports()
    }
}

// private functions
impl<D: DictionaryAccess> DictBuilder<D> {
    fn set_user(&mut self, user: bool) {
        if user {
            self.header.version = HeaderVersion::UserDict(UserDictVersion::Version3)
        } else {
            self.header.version = HeaderVersion::SystemDict(SystemDictVersion::Version2)
        }
        self.user = user;
    }

    fn write_grammar<W: Write>(&mut self, w: &mut W) -> SudachiResult<usize> {
        let mut size = 0;
        let r1 = ReportBuilder::new("pos_table");
        size += self.lexicon.write_pos_table(w)?;
        self.reporter.collect(size, r1);
        let r2 = ReportBuilder::new("conn_matrix");
        size += self.conn.write_to(w)?;
        self.reporter.collect(size, r2);
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

        let report = ReportBuilder::new("trie");
        let word_id_table = index.build_word_id_table()?;
        let trie = index.build_trie()?;

        let trie_size = trie.len() / 4;
        w.write_all(&(trie_size as u32).to_le_bytes())?;
        size += 4;
        w.write_all(&trie)?;
        size += trie.len();
        std::mem::drop(trie); //can be big, so drop explicitly
        self.reporter.collect(size, report);
        let cur_size = size;

        let report = ReportBuilder::new("word_id table");
        w.write_all(&(word_id_table.len() as u32).to_le_bytes())?;
        size += 4;
        w.write_all(&word_id_table)?;
        size += word_id_table.len();
        self.reporter.collect(size - cur_size, report);

        Ok(size)
    }

    fn write_lexicon<W: Write>(&mut self, w: &mut W, offset: usize) -> SudachiResult<usize> {
        let mut size = self.write_index(w)?;
        let mut writer =
            LexiconWriter::new(self.lexicon.entries(), offset + size, &mut self.reporter);
        size += writer.write(w)?;
        Ok(size)
    }

    fn check_if_resolved(&self) -> SudachiResult<()> {
        if self.lexicon.needs_split_resolution() && !self.resolved {
            return self.ctx.err(BuildFailure::UnresolvedSplits);
        }

        Ok(())
    }

    /// this function must only be used in resolve_impl
    fn unsafe_make_resolver<'a, 'b>(&'a self) -> RawDictResolver<'b> {
        let resolver = RawDictResolver::new(self.lexicon.entries(), self.user);
        // resolver borrows parts of entries, but it does not touch splits
        // resolve function only modifies splits
        unsafe { std::mem::transmute(resolver) }
    }

    fn resolve_impl(&mut self) -> SudachiResult<usize> {
        if !self.lexicon.needs_split_resolution() {
            self.resolved = true;
            return Ok(0);
        }

        let this_resolver = self.unsafe_make_resolver();
        let report = ReportBuilder::new("resolve");

        let cnt = match self.prebuilt.as_ref() {
            Some(d) => {
                let built_resolver = BinDictResolver::new(d)?;
                let chained = ChainedResolver::new(this_resolver, built_resolver);
                self.lexicon.resolve_splits(&chained)
            }
            None => self.lexicon.resolve_splits(&this_resolver),
        };
        let cnt = self.reporter.collect_r(cnt, report);
        match cnt {
            Ok(cnt) => {
                self.resolved = true;
                Ok(cnt)
            }
            Err((split_info, line)) => Err(DicBuildError {
                file: "<entries>".to_owned(),
                line,
                cause: BuildFailure::InvalidSplitWordReference(split_info),
            }
            .into()),
        }
    }
}
