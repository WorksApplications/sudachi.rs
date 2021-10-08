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

use std::fmt::Debug;
use thiserror::Error;

use crate::config::ConfigError;
use crate::dic::character_category::Error as CharacterCategoryError;
use crate::dic::header::HeaderError;
use crate::dic::lexicon_set::LexiconSetError;
use crate::plugin::PluginError;

pub type SudachiResult<T> = Result<T, SudachiError>;

/// Sudachi error
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum SudachiError {
    #[error("IO Error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse Int Error")]
    ParseIntError(#[from] std::num::ParseIntError),

    #[error("Invalid UTF-16: {0}")]
    FromUtf16(#[from] std::string::FromUtf16Error),

    #[error("Regex error")]
    RegexError(#[from] fancy_regex::Error),

    #[error("Libloading Error: {0}")]
    Libloading(#[from] libloading::Error),

    #[error("Error from nom {0}")]
    NomParseError(String),

    #[error("Invalid utf16 string from nom")]
    InvalidUtf16FromNom,

    #[error("Serde error: {0}")]
    SerdeError(#[from] serde_json::Error),

    #[error("Invalid character category definition: {0}")]
    InvalidCharacterCategory(#[from] CharacterCategoryError),

    #[error("Config Error: {0}")]
    ConfigError(#[from] ConfigError),

    #[error("Invalid header: {0}")]
    InvalidHeader(#[from] HeaderError),

    #[error("Lecicon error")]
    LexiconSetError(#[from] LexiconSetError),

    #[error("Plugin error")]
    PluginError(#[from] PluginError),

    #[error("End of sentence (EOS) is not connected to beginning of sentence (BOS)")]
    EosBosDisconnect,

    #[error("Invalid character category type: {0}")]
    InvalidCharacterCategoryType(String),

    #[error("Invalid data format: {1} at line {0}")]
    InvalidDataFormat(usize, String),

    #[error("Invalid grammar")]
    InvalidDictionaryGrammar,

    #[error("Invalid part of speech: {0}")]
    InvalidPartOfSpeech(String),

    #[error("Invalid range: {0}..{1}")]
    InvalidRange(usize, usize),

    #[error("Missing dictionary trie")]
    MissingDictionaryTrie,

    #[error("Missing latice path")]
    MissingLaticePath,

    #[error("Missing part of speech")]
    MissingPartOfSpeech,

    #[error("Missing word_id")]
    MissingWordId,

    #[error("Missing word_info")]
    MissingWordInfo,

    #[error("No out of vocabulary plugin provided")]
    NoOOVPluginProvided,

    #[error("Input is too long, it can't be more than {1} bytes, was {0}")]
    InputTooLong(usize, usize),
}

pub type SudachiNomResult<I, O> = nom::IResult<I, O, SudachiNomError<I>>;

/// Custum nom error
#[derive(Debug, PartialEq)]
pub enum SudachiNomError<I> {
    /// Failed to parse utf16 string
    Utf16String,
    Nom(I, nom::error::ErrorKind),
}

impl<I> nom::error::ParseError<I> for SudachiNomError<I> {
    fn from_error_kind(input: I, kind: nom::error::ErrorKind) -> Self {
        SudachiNomError::Nom(input, kind)
    }
    fn append(_: I, _: nom::error::ErrorKind, other: Self) -> Self {
        other
    }
}

impl<I: Debug> From<nom::Err<SudachiNomError<I>>> for SudachiError {
    fn from(err: nom::Err<SudachiNomError<I>>) -> Self {
        if let nom::Err::Failure(SudachiNomError::Utf16String) = err {
            return SudachiError::InvalidUtf16FromNom;
        }
        SudachiError::NomParseError(format!("{}", err))
    }
}
