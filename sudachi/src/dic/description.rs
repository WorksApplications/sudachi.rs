/*
 * Copyright (c) 2025-2026 Works Applications Co., Ltd.
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

use nom::number::complete::le_u64;
use std::fmt::Display;
use std::time::Duration;
use thiserror::Error;

use super::header::HeaderVersion;
use super::read::{
    error::SudachiNomResult,
    utf8_string::utf8_string,
    varint::{varint32, varint64},
};
use crate::error::SudachiResult;

static MAGIC_BYTES: &[u8] = b"SudachiBinaryDic";

/// Sudachi error
#[derive(Error, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum DescriptionError {
    #[error("Unable to parse")]
    CannotParse,

    #[error("Invalid magic bytes")]
    InvalidMagicBytes,

    #[error("V0 version")]
    V0Version,

    #[error("Invalid header version {0}")]
    InvalidVersion(u64),

    #[error("Dictionary part not found: {0}")]
    DictionaryPartNotFound(String),

    #[error("Dictionary part out of range: {0}..{1}")]
    DictionaryPartOutOfRange(usize, usize),
}

/// Parts of the binary dictionary
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Block {
    // description always takes the first 1 block
    // grammar parts:
    // word connection cost matrix
    ConnectionMatrix,
    // list of part of speechs
    POSTable,
    // lexicon parts:
    // TRIE
    TRIEIndex,
    // mapping from a index-form to entries
    WordPointers,
    // word entries
    Entries,
    // storage of strings in the lexicon
    Strings,
    // build-time reference_id table
    ReferenceIdTable,
}

impl Block {
    /// return the string representation of the block
    /// This must be same as the name defined in the Java version.
    fn as_string_representation(&self) -> &str {
        match self {
            Block::ConnectionMatrix => "ConnMatrix",
            Block::POSTable => "POS",
            Block::TRIEIndex => "TrieIndex",
            Block::WordPointers => "WordPointers",
            Block::Entries => "Entries",
            Block::Strings => "Strings",
            Block::ReferenceIdTable => "ReferenceIdTable",
        }
    }
}

impl Display for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_string_representation().to_string())
    }
}

/// Information of each blocks in the binary dictionary
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockInfo {
    name: String,
    start: u64,
    size: u64,
}

impl BlockInfo {
    /// Parse BlockInfo from bytes.
    pub fn parse(buf: &[u8]) -> SudachiNomResult<&[u8], Self> {
        let (rest, (name, start, size)) =
            nom::sequence::tuple((utf8_string, varint64, varint64))(buf)?;
        Ok((rest, Self { name, start, size }))
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn start(&self) -> u64 {
        self.start
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn end(&self) -> u64 {
        self.start + self.size
    }
}

/// The description part of the binary dictionary
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Description {
    creation_time: Duration,
    comment: String,
    signature: String,
    reference: String,
    blocks: Vec<BlockInfo>,
    flags: u64,
    num_total_entries: u32,
    num_indexed_entries: u32,
}

impl Description {
    pub fn load(buf: &[u8]) -> SudachiResult<Self> {
        Self::check_v0_format(buf)?;

        let rest = Self::check_magic(buf)?;
        let (rest, version) = le_u64(rest)?;
        if version == 1 {
            Self::load_v1(rest)
        } else {
            Err(DescriptionError::InvalidVersion(version).into())
        }
    }

    /// Check if the dictionary is in V0 format.
    ///
    /// In V0 format, the version is stored as a u64 (long) at the beginning of the binary.
    fn check_v0_format(buf: &[u8]) -> SudachiResult<()> {
        let (_rest, version) = le_u64(buf)?;
        let v0_version = HeaderVersion::from_u64(version);
        match v0_version {
            Some(_) => Err(DescriptionError::V0Version.into()),
            None => Ok(()),
        }
    }

    /// Check if the first bytes are magic bytes.
    ///
    /// If so, return the rest of the buffer.
    fn check_magic(buf: &[u8]) -> SudachiResult<&[u8]> {
        let (rest, first_bytes) = nom::bytes::complete::take(MAGIC_BYTES.len())(buf)?;
        match MAGIC_BYTES
            .iter()
            .zip(first_bytes.iter())
            .position(|(a, b)| a != b)
        {
            Some(_) => Err(DescriptionError::InvalidMagicBytes.into()),
            None => Ok(rest),
        }
    }

    /// Load Vi format description.
    ///
    /// assume that followings are already loaded:
    /// - magic bytes (16 bytes)
    /// - version (long/u64)
    fn load_v1(buf: &[u8]) -> SudachiResult<Self> {
        let (
            _rest,
            (
                creation_time_secs,
                flags,
                comment,
                signature,
                reference,
                num_indexed_entries,
                num_total_entries,
                blocks,
            ),
        ) = nom::sequence::tuple((
            le_u64,
            le_u64,
            utf8_string,
            utf8_string,
            utf8_string,
            varint32,
            varint32,
            nom::multi::length_count(varint32, BlockInfo::parse),
        ))(buf)?;

        Ok(Self {
            creation_time: Duration::from_secs(creation_time_secs),
            comment,
            signature,
            reference,
            blocks,
            flags,
            num_total_entries,
            num_indexed_entries,
        })
    }

    pub fn creation_time(&self) -> Duration {
        self.creation_time
    }

    pub fn comment(&self) -> &str {
        &self.comment
    }

    pub fn signature(&self) -> &str {
        &self.signature
    }

    pub fn reference(&self) -> &str {
        &self.reference
    }

    pub fn is_system_dictionary(&self) -> bool {
        self.reference.is_empty()
    }

    pub fn is_user_dictionary(&self) -> bool {
        !self.reference.is_empty()
    }

    pub fn blocks(&self) -> &[BlockInfo] {
        &self.blocks
    }

    pub fn slice_or_none<'a>(
        &self,
        buf: &'a [u8],
        block: Block,
    ) -> SudachiResult<Option<&'a [u8]>> {
        let block_name = block.as_string_representation();
        match self.blocks.iter().find(|block| block.name() == block_name) {
            Some(block) => {
                let start = block.start() as usize;
                let end = block.end() as usize;
                if buf.len() < end {
                    return Err(DescriptionError::DictionaryPartOutOfRange(start, end).into());
                }
                Ok(Some(&buf[start..end]))
            }
            None => Ok(None),
        }
    }

    pub fn slice<'a>(&self, buf: &'a [u8], block: Block) -> SudachiResult<&'a [u8]> {
        self.slice_or_none(buf, block)?
            .ok_or_else(|| DescriptionError::DictionaryPartNotFound(block.to_string()).into())
    }

    pub fn is_runtime_costs(&self) -> bool {
        self.flags & 0x1 != 0
    }

    pub fn num_total_entries(&self) -> u32 {
        self.num_total_entries
    }

    pub fn num_indexed_entries(&self) -> u32 {
        self.num_indexed_entries
    }
}
