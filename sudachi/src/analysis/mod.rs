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

use std::fmt::{Display, Formatter};
use std::str::FromStr;

use mlist::MorphemeList;

use crate::error::SudachiResult;

mod inner;
pub mod lattice;
pub mod mlist;
pub mod morpheme;
pub mod node;
pub mod stateful_tokenizer;
pub mod stateless_tokenizer;

pub use inner::Node;

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

impl Display for Mode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let repr = match self {
            Mode::A => "A",
            Mode::B => "B",
            Mode::C => "C",
        };
        f.write_str(repr)
    }
}

/// Able to tokenize Japanese text
pub trait Tokenize {
    type Dictionary;

    /// Break text into `Morpheme`s
    fn tokenize(
        &self,
        input: &str,
        mode: Mode,
        enable_debug: bool,
    ) -> SudachiResult<MorphemeList<Self::Dictionary>>;
}
