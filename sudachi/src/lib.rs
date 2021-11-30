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

//! Clone of [Sudachi](https://github.com/WorksApplications/Sudachi),
//! a Japanese morphological analyzer
//!
//! There is no public API for the initial release.
//! Issue: https://github.com/WorksApplications/sudachi.rs/issues/28
//!
//! Also, there are to mostly
//! [SudachiPy-compatible Python bindings](https://worksapplications.github.io/sudachi.rs/python/).

pub mod analysis;
pub mod config;
pub mod dic;
pub mod error;
pub mod input_text;
pub mod plugin;
pub mod sentence_detector;
pub mod sentence_splitter;
pub(crate) mod util;

mod hash;
pub mod pos;
#[cfg(test)]
pub mod test;

pub mod prelude {
    pub use crate::{
        analysis::mlist::MorphemeList, analysis::morpheme::Morpheme, analysis::Mode,
        error::SudachiError, error::SudachiResult,
    };
}
