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

pub mod created;
mod inner;
pub mod lattice;
pub mod mlist;
pub mod mode;
pub mod morpheme;
pub mod node;
pub mod stateful_tokenizer;
pub mod stateless_tokenizer;

pub use inner::Node;
pub use mode::Mode;

use crate::analysis::mlist::MorphemeList;
use crate::error::SudachiResult;

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
