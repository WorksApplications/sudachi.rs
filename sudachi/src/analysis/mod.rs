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

use std::str::FromStr;

use morpheme_list::MorphemeList;

use crate::error::SudachiResult;

pub mod lattice;
pub mod morpheme;
pub mod morpheme_list;
pub mod node;
pub mod stateless_tokenizer;

/// Unit to split text
///
/// Some examples:
/// ```text
/// A：選挙/管理/委員/会
/// B：選挙/管理/委員会
/// C：選挙管理委員会
///
/// A：客室/乗務/員
/// B：客室/乗務員
/// C：客室乗務員
///
/// A：労働/者/協同/組合
/// B：労働者/協同/組合
/// C：労働者協同組合
///
/// A：機能/性/食品
/// B：機能性/食品
/// C：機能性食品
/// ```
///
/// See [Sudachi documentation](https://github.com/WorksApplications/Sudachi#the-modes-of-splitting)
/// for more details
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Short
    A,

    /// Middle (similar to "word")
    B,

    /// Named Entity
    C,
}

impl FromStr for Mode {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "A" | "a" => Ok(Mode::A),
            "B" | "b" => Ok(Mode::B),
            "C" | "c" => Ok(Mode::C),
            _ => Err("Mode must be one of \"A\", \"B\", or \"C\" (in lower or upper case)."),
        }
    }
}

/// Able to tokenize Japanese text
pub trait Tokenize {
    type Dictionary: ?Sized;

    /// Break text into `Morpheme`s
    fn tokenize<'a>(
        &'a self,
        input: &'a str,
        mode: Mode,
        enable_debug: bool,
    ) -> SudachiResult<MorphemeList<&'a Self::Dictionary>>;

    /// Split text into sentences then tokenize
    fn tokenize_sentences<'a>(
        &'a self,
        input: &'a str,
        mode: Mode,
        enable_debug: bool,
    ) -> SudachiResult<Vec<MorphemeList<&'a Self::Dictionary>>>;
}
