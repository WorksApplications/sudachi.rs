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

use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::dic::build::error::{BuildFailure, DicBuildError, DicCompilationCtx};
use crate::dic::build::index::IndexBuilder;
use crate::dic::build::lexicon::{LexiconWriter, StringStore};
use crate::dic::build::report::{DictPartReport, ReportBuilder, Reporter};
use crate::dic::build::resolve::{BinDictResolver, ChainedResolver, RawDictResolver};
use crate::dic::build::util::default_signature;
use crate::dic::description::Block;
use crate::dic::grammar::Grammar;
use crate::dic::lexicon_set::LexiconSet;
use crate::dic::word_id::WordId;
use crate::dic::{DescriptionAccess, DictionaryAccess, LexiconAccess, ReferenceIdAccess};
use crate::error::SudachiResult;
use crate::plugin::input_text::InputTextPlugin;
use crate::plugin::oov::OovProviderPlugin;
use crate::plugin::path_rewrite::PathRewritePlugin;

pub(crate) mod conn;
pub(crate) mod csv_schema;
pub mod error;
pub(crate) mod index;
pub(crate) mod lexicon;
pub(crate) mod parse;
pub(crate) mod pos;
pub mod report;
mod resolve;
#[cfg(test)]
mod test;
mod util;

pub use self::index::{CacheAwareOptions, LayoutScoring, TrieBuildStrategy, TrieProfileMode};

const MAX_POS_IDS: usize = i16::MAX as usize;
const MAX_DIC_STRING_LEN: usize = i16::MAX as usize;
const MAX_ARRAY_LEN: usize = i8::MAX as usize;
const DICT_BLOCK_SIZE: usize = 4096;
const DESCRIPTION_MAGIC_BYTES: &[u8] = b"SudachiBinaryDic";
const DESCRIPTION_VERSION: u64 = 1;
const DEFAULT_USER_REFERENCE: &str = "system.dic";

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

#[derive(Copy, Clone, Eq, PartialEq)]
enum BuilderStage {
    Grammar,
    Lexicon,
    Resolved,
}

impl LexiconAccess for NoDic {
    fn lexicon(&self) -> &LexiconSet<'_> {
        panic!("there is no lexicon here")
    }
}

impl DictionaryAccess for NoDic {
    fn grammar(&self) -> &Grammar<'_> {
        panic!("there is no grammar here")
    }

    fn input_text_plugins(&self) -> &[Box<dyn InputTextPlugin + Sync + Send>] {
        &[]
    }

    fn oov_provider_plugins(&self) -> &[Box<dyn OovProviderPlugin + Sync + Send>] {
        &[]
    }

    fn path_rewrite_plugins(&self) -> &[Box<dyn PathRewritePlugin + Sync + Send>] {
        &[]
    }
}

impl ReferenceIdAccess for NoDic {
    fn reference_ids(&self) -> std::collections::HashMap<u32, String> {
        std::collections::HashMap::new()
    }
}

/// Builds a binary dictionary from csv lexicon and connection matrix (optional)
pub struct DictBuilder<D> {
    user: bool,
    lexicon: lexicon::LexiconReader,
    conn: conn::ConnBuffer,
    ctx: DicCompilationCtx,
    compile_time: SystemTime,
    description: String,
    signature: String,
    reference: String,
    stage: BuilderStage,
    prebuilt: Option<D>,
    reporter: Reporter,
    trie_build_strategy: TrieBuildStrategy,
}

impl DictBuilder<NoDic> {
    /// Creates a new builder for system dictionary
    pub fn new_system() -> Self {
        Self::new_empty()
    }
}

impl<D: DictionaryAccess + ReferenceIdAccess> DictBuilder<D> {
    fn new_empty() -> Self {
        Self {
            user: false,
            lexicon: lexicon::LexiconReader::new(),
            conn: conn::ConnBuffer::new(),
            ctx: DicCompilationCtx::default(),
            compile_time: SystemTime::now(),
            description: String::new(),
            signature: String::new(),
            reference: String::new(),
            stage: BuilderStage::Grammar,
            prebuilt: None,
            reporter: Reporter::new(),
            trie_build_strategy: TrieBuildStrategy::default(),
        }
    }
}

impl<D: DictionaryAccess + DescriptionAccess + ReferenceIdAccess> DictBuilder<D> {
    /// Creates a new builder for user dictionary
    pub fn new_user(system: D) -> Self {
        let mut bldr = Self::new_empty();
        bldr.set_user(true);
        let cm = system.grammar().conn_matrix();
        bldr.lexicon
            .set_max_conn_sizes(cm.num_left() as _, cm.num_right() as _);
        bldr.lexicon.preload_pos(system.grammar());
        let max_system_entry_id = system
            .lexicon()
            .system_word_ids_in_order()
            .into_iter()
            .map(|wid| wid.entry().as_raw() as usize)
            .max()
            .unwrap_or(usize::MAX);
        bldr.lexicon.set_max_system_entry_id(max_system_entry_id);
        let signature = system.description().signature();
        if !signature.is_empty() {
            bldr.reference = signature.to_owned();
        }
        bldr.prebuilt = Some(system);
        bldr
    }
}

impl<D: DictionaryAccess + ReferenceIdAccess> DictBuilder<D> {
    /// Set the dictionary compile time to the specified time instead of current time
    pub fn set_compile_time<T: Into<std::time::SystemTime>>(
        &mut self,
        time: T,
    ) -> std::time::SystemTime {
        std::mem::replace(&mut self.compile_time, time.into())
    }

    /// Set the dictionary description
    pub fn set_description<T: Into<String>>(&mut self, description: T) {
        self.description = description.into()
    }

    /// Set the trie layout strategy used while compiling the dictionary.
    ///
    /// The default strategy is [`TrieBuildStrategy::ClassicYada`], preserving
    /// the current dictionary bytes unless callers explicitly opt in to a
    /// different layout.
    pub fn set_trie_build_strategy(&mut self, strategy: TrieBuildStrategy) {
        self.trie_build_strategy = strategy;
    }

    /// Read the connection matrix from either a file or an in-memory buffer
    ///
    /// This API is intended for system dictionary builds.
    pub fn read_conn<'a, T: AsDataSource<'a> + 'a>(&mut self, data: T) -> SudachiResult<()> {
        self.ensure_grammar_stage(
            "read_conn() must be called before reading lexicon or resolving",
        )?;
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

    /// Read POS table csv from either a file or an in-memory buffer.
    ///
    /// This API is intended for system dictionary builds.
    pub fn read_pos<'a, T: AsDataSource<'a> + 'a>(&mut self, data: T) -> SudachiResult<usize> {
        if self.user {
            return self.ctx.err(BuildFailure::InvalidSplit(
                "read_pos is not available for user dictionary".to_owned(),
            ));
        }
        self.ensure_grammar_stage("read_pos() must be called before reading lexicon or resolving")?;

        let report = ReportBuilder::new(data.name()).read();
        let result = match data.convert() {
            DataSource::File(p) => self.lexicon.read_pos_file(p),
            DataSource::Data(d) => self.lexicon.read_pos_bytes(d),
        };
        self.reporter.collect_r(result, report)
    }

    /// Read the csv lexicon from either a file or an in-memory buffer
    pub fn read_lexicon<'a, T: AsDataSource<'a> + 'a>(&mut self, data: T) -> SudachiResult<usize> {
        self.ensure_lexicon_stage()?;
        let report = ReportBuilder::new(data.name()).read();
        let result = match data.convert() {
            DataSource::File(p) => self.lexicon.read_file(p),
            DataSource::Data(d) => self.lexicon.read_bytes(d),
        };
        let result = self.reporter.collect_r(result, report);
        if result.is_ok() {
            self.stage = BuilderStage::Lexicon;
        }
        result
    }

    /// Resolve the dictionary references.
    ///
    /// Returns the number of resolved entries
    pub fn resolve(&mut self) -> SudachiResult<usize> {
        self.ensure_resolve_stage()?;
        self.resolve_impl()
    }

    /// Compile the binary dictionary and write it to the specified sink
    pub fn compile<W: Write>(&mut self, w: &mut W) -> SudachiResult<()> {
        self.prepare_description_fields();
        self.ensure_compile_stage()?;

        let mut buffer = vec![0u8; DICT_BLOCK_SIZE];
        let mut blocks: Vec<BlockInfo> = Vec::with_capacity(7);

        if !self.user {
            self.align_to_block(&mut buffer);
            let start = buffer.len();
            let report = ReportBuilder::new("conn_matrix");
            let size = self.conn.write_to(&mut buffer)?;
            self.reporter.collect(size, report);
            blocks.push(BlockInfo::new(Block::ConnectionMatrix, start, size));
        }

        self.align_to_block(&mut buffer);
        let start = buffer.len();
        let report = ReportBuilder::new("pos_table");
        let size = self.lexicon.write_pos_table(&mut buffer)?;
        self.reporter.collect(size, report);
        blocks.push(BlockInfo::new(Block::POSTable, start, size));

        let (trie, word_id_table) = self.build_index_data()?;
        let strings = StringStore::from_entries(self.lexicon.resolved_entries())?;

        self.align_to_block(&mut buffer);
        let start = buffer.len();
        let report = ReportBuilder::new("word_id table");
        buffer.write_all(&word_id_table)?;
        self.reporter.collect(word_id_table.len(), report);
        blocks.push(BlockInfo::new(
            Block::WordPointers,
            start,
            word_id_table.len(),
        ));

        self.align_to_block(&mut buffer);
        let start = buffer.len();
        let report = ReportBuilder::new("trie");
        buffer.write_all(&trie)?;
        self.reporter.collect(trie.len(), report);
        blocks.push(BlockInfo::new(Block::TRIEIndex, start, trie.len()));

        self.align_to_block(&mut buffer);
        let start = buffer.len();
        let report = ReportBuilder::new("strings");
        let size = strings.write(&mut buffer)?;
        self.reporter.collect(size, report);
        blocks.push(BlockInfo::new(Block::Strings, start, size));

        self.align_to_block(&mut buffer);
        let start = buffer.len();
        let mut writer = LexiconWriter::new(
            self.lexicon.resolved_entries(),
            &strings,
            self.user,
            &mut self.reporter,
        );
        let size = writer.write(&mut buffer)?;
        blocks.push(BlockInfo::new(Block::Entries, start, size));

        self.align_to_block(&mut buffer);
        let start = buffer.len();
        let report = ReportBuilder::new("reference_id_table");
        let size = self.write_reference_id_table(&mut buffer)?;
        self.reporter.collect(size, report);
        blocks.push(BlockInfo::new(Block::ReferenceIdTable, start, size));

        let runtime_costs = self
            .lexicon
            .resolved_entries()
            .iter()
            .any(|e| e.cost == i16::MIN);
        // phantom entries stay serialized for reference resolution,
        // but they are excluded from the public entry counts in the description metadata.
        let num_total_entries = self
            .lexicon
            .resolved_entries()
            .iter()
            .filter(|e| !e.is_phantom())
            .count() as u32;
        let num_indexed_entries = self
            .lexicon
            .resolved_entries()
            .iter()
            .filter(|e| !e.is_phantom() && e.should_index())
            .count() as u32;
        let description = self.serialize_description(
            &blocks,
            num_indexed_entries,
            num_total_entries,
            runtime_costs,
        )?;
        buffer[..description.len()].copy_from_slice(&description);

        w.write_all(&buffer)?;
        Ok(())
    }

    /// Return dictionary build report
    pub fn report(&self) -> &[DictPartReport] {
        self.reporter.reports()
    }
}

// private functions
impl<D: DictionaryAccess + ReferenceIdAccess> DictBuilder<D> {
    fn set_user(&mut self, user: bool) {
        if user && self.reference.is_empty() {
            self.reference = DEFAULT_USER_REFERENCE.to_owned();
        }
        if !user {
            self.reference.clear();
        }
        self.user = user;
    }

    fn make_resolver(&self) -> SudachiResult<RawDictResolver> {
        let line_to_wref = self.lexicon.row_word_refs(self.user);
        self.ctx.transform(RawDictResolver::new(
            self.lexicon.entries(),
            line_to_wref,
            self.user,
        ))
    }

    fn resolve_impl(&mut self) -> SudachiResult<usize> {
        let this_resolver = self.make_resolver()?;
        let report = ReportBuilder::new("resolve");

        let cnt = match self.prebuilt.as_ref() {
            Some(d) => {
                let built_resolver = BinDictResolver::new(d)?;
                let chained = ChainedResolver::new(this_resolver, built_resolver);
                self.lexicon.resolve_entries(&chained, self.user)
            }
            None => self.lexicon.resolve_entries(&this_resolver, self.user),
        };
        let cnt = self.reporter.collect_r(cnt, report);
        match cnt {
            Ok(cnt) => {
                self.stage = BuilderStage::Resolved;
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

    /// Set signature.
    /// System dictionary has a signature string and user dictionary has empty string.
    fn prepare_description_fields(&mut self) {
        if self.user {
            self.signature.clear();
        } else if self.signature.is_empty() {
            self.signature = default_signature(self.compile_time, &self.description);
        }
    }

    fn ensure_grammar_stage(&self, message: &'static str) -> SudachiResult<()> {
        if self.stage != BuilderStage::Grammar {
            return self.ctx.err(BuildFailure::InvalidBuilderState(message));
        }
        Ok(())
    }

    fn ensure_lexicon_stage(&self) -> SudachiResult<()> {
        if self.stage == BuilderStage::Resolved {
            return self.ctx.err(BuildFailure::InvalidBuilderState(
                "read_lexicon() must be called before resolve()",
            ));
        }
        Ok(())
    }

    fn ensure_resolve_stage(&self) -> SudachiResult<()> {
        match self.stage {
            BuilderStage::Grammar => self.ctx.err(BuildFailure::InvalidBuilderState(
                "resolve() must be called after reading lexicon",
            )),
            BuilderStage::Lexicon => Ok(()),
            BuilderStage::Resolved => self.ctx.err(BuildFailure::InvalidBuilderState(
                "resolve() cannot be called more than once",
            )),
        }
    }

    fn ensure_compile_stage(&self) -> SudachiResult<()> {
        if self.stage != BuilderStage::Resolved {
            return self.ctx.err(BuildFailure::InvalidBuilderState(
                "compile() must be called after resolve()",
            ));
        }
        Ok(())
    }

    fn align_to_block(&self, buffer: &mut Vec<u8>) {
        let rem = buffer.len() % DICT_BLOCK_SIZE;
        if rem != 0 {
            buffer.resize(buffer.len() + (DICT_BLOCK_SIZE - rem), 0);
        }
    }

    fn build_index_data(&mut self) -> SudachiResult<(Vec<u8>, Vec<u8>)> {
        let mut index = IndexBuilder::with_trie_build_strategy(self.trie_build_strategy.clone());
        self.fill_index_builder(&mut index)?;
        let word_id_table = index.build_word_id_table(&self.non_indexed_word_ids())?;
        let trie = index.build_trie()?;
        Ok((trie, word_id_table))
    }

    fn fill_index_builder<'a>(&'a self, index: &mut IndexBuilder<'a>) -> SudachiResult<()> {
        let entry_ids = self.lexicon.row_word_ids(0);
        for (e, wid) in self
            .lexicon
            .resolved_entries()
            .iter()
            .zip(entry_ids.into_iter())
        {
            if e.should_index() {
                index.add(e.index_form(), wid);
            }
        }
        Ok(())
    }

    fn non_indexed_word_ids(&self) -> Vec<WordId> {
        // Keep non-indexed, non-phantom entries in the word-id table as a
        // trailing list. This preserves compatibility with the Java dictionary
        // format, where callers can enumerate all public entries from
        // WordIdTable even if some of them are intentionally absent from the
        // trie. Phantom entries stay internal to reference resolution.
        let entry_ids = self.lexicon.row_word_ids(0);
        self.lexicon
            .resolved_entries()
            .iter()
            .zip(entry_ids)
            .filter_map(|(entry, word_id)| {
                (!entry.should_index() && !entry.is_phantom()).then_some(word_id)
            })
            .collect()
    }

    fn serialize_description(
        &self,
        blocks: &[BlockInfo],
        num_indexed_entries: u32,
        num_total_entries: u32,
        runtime_costs: bool,
    ) -> SudachiResult<Vec<u8>> {
        let mut out = Vec::with_capacity(DICT_BLOCK_SIZE);
        out.extend_from_slice(DESCRIPTION_MAGIC_BYTES);
        out.extend_from_slice(&DESCRIPTION_VERSION.to_le_bytes());

        let secs = self
            .compile_time
            .duration_since(UNIX_EPOCH)
            .map_err(|_| self.ctx.to_sudachi_err(BuildFailure::InvalidCompileTime))?
            .as_secs();
        out.extend_from_slice(&secs.to_le_bytes());
        let flags = if runtime_costs { 1u64 } else { 0u64 };
        out.extend_from_slice(&flags.to_le_bytes());
        self.put_utf8_string(&mut out, &self.description)?;
        self.put_utf8_string(&mut out, &self.signature)?;
        self.put_utf8_string(&mut out, &self.reference)?;
        Self::put_varint(&mut out, num_indexed_entries as u64);
        Self::put_varint(&mut out, num_total_entries as u64);
        Self::put_varint(&mut out, blocks.len() as u64);
        for block in blocks {
            self.put_utf8_string(&mut out, &block.name)?;
            Self::put_varint(&mut out, block.start as u64);
            Self::put_varint(&mut out, block.size as u64);
        }
        if out.len() > DICT_BLOCK_SIZE {
            return self.ctx.err(BuildFailure::InvalidSize {
                actual: out.len(),
                expected: DICT_BLOCK_SIZE,
            });
        }
        Ok(out)
    }

    fn put_utf8_string(&self, dst: &mut Vec<u8>, data: &str) -> SudachiResult<()> {
        let length = u32::try_from(data.len()).map_err(|_| {
            self.ctx.to_sudachi_err(BuildFailure::InvalidSize {
                actual: data.len(),
                expected: u32::MAX as usize,
            })
        })?;
        Self::put_varint(dst, length as u64);
        dst.extend_from_slice(data.as_bytes());
        Ok(())
    }

    fn put_varint(dst: &mut Vec<u8>, mut value: u64) {
        loop {
            let mut byte = (value & 0x7f) as u8;
            value >>= 7;
            if value != 0 {
                byte |= 0x80;
            }
            dst.push(byte);
            if value == 0 {
                break;
            }
        }
    }

    fn write_reference_id_table<W: Write>(&self, dst: &mut W) -> SudachiResult<usize> {
        let mut out = Vec::new();
        let mut rows = Vec::new();
        let mut offset = lexicon::LexiconReader::ENTRY_INITIAL_OFFSET;
        for entry in self.lexicon.resolved_entries() {
            let entry_id =
                (offset >> crate::dic::word_info::WordInfos::WORD_ID_ALIGNMENT_BITS) as u32;
            if !entry.is_phantom() {
                if let Some(reference_id) = entry.reference_id() {
                    rows.push((entry_id, reference_id));
                }
            }
            offset += entry.expected_entry_size();
        }
        Self::put_varint(&mut out, rows.len() as u64);
        for (entry_id, reference_id) in rows {
            Self::put_varint(&mut out, entry_id as u64);
            self.put_utf8_string(&mut out, reference_id)?;
        }
        dst.write_all(&out)?;
        Ok(out.len())
    }
}

struct BlockInfo {
    name: String,
    start: usize,
    size: usize,
}

impl BlockInfo {
    fn new(block: Block, start: usize, size: usize) -> Self {
        Self {
            name: block.to_string(),
            start,
            size,
        }
    }
}
