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

use libloading::Error as LLError;
use thiserror::Error;

use crate::config::Config;
use crate::dic::grammar::Grammar;
use crate::plugin::connect_cost::EditConnectionCostPlugin;
use crate::plugin::input_text::InputTextPlugin;
use crate::plugin::loader::{load_plugins_of, PluginContainer};
use crate::plugin::oov::OovProviderPlugin;
use crate::plugin::path_rewrite::PathRewritePlugin;
use crate::prelude::*;

pub use self::loader::PluginCategory;

pub mod connect_cost;
pub mod dso;
pub mod input_text;
mod loader;
pub mod oov;
pub mod path_rewrite;

#[derive(Error, Debug)]
pub enum PluginError {
    #[error("IO Error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Libloading Error: {message} ; {source}")]
    Libloading { source: LLError, message: String },

    #[error("Serde error: {0}")]
    SerdeError(#[from] serde_json::Error),

    #[error("Invalid data format: {0}")]
    InvalidDataFormat(String),
}

impl From<LLError> for PluginError {
    fn from(e: LLError) -> Self {
        PluginError::Libloading {
            source: e,
            message: String::new(),
        }
    }
}

pub(crate) struct Plugins {
    pub(crate) connect_cost: PluginContainer<dyn EditConnectionCostPlugin>,
    pub(crate) input_text: PluginContainer<dyn InputTextPlugin>,
    pub(crate) oov: PluginContainer<dyn OovProviderPlugin>,
    pub(crate) path_rewrite: PluginContainer<dyn PathRewritePlugin>,
}

impl Plugins {
    pub(crate) fn load(cfg: &Config, grammar: &Grammar) -> SudachiResult<Plugins> {
        let plugins = Plugins {
            connect_cost: load_plugins_of(cfg, grammar)
                .map_err(|e| e.with_context("connect_cost"))?,
            input_text: load_plugins_of(cfg, grammar).map_err(|e| e.with_context("input_text"))?,
            oov: load_plugins_of(cfg, grammar).map_err(|e| e.with_context("oov"))?,
            path_rewrite: load_plugins_of(cfg, grammar)
                .map_err(|e| e.with_context("path_rewrite"))?,
        };
        Ok(plugins)
    }
}
