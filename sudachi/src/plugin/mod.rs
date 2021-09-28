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

use std::ffi::OsStr;
use std::path::PathBuf;

use libloading::Error as LLError;
use serde_json::Value;
use thiserror::Error;

use crate::config::{Config, ConfigError};
use crate::config::ConfigError::FileNotFound;
use crate::dic::grammar::Grammar;
use crate::plugin::connect_cost::{
    EditConnectionCostPluginManager, get_edit_connection_cost_plugins,
};
use crate::plugin::input_text::InputTextPlugin;
use crate::plugin::loader::{load_plugins_of, PluginContainer};
use crate::plugin::oov::{get_oov_plugins, OovProviderPluginManager};
use crate::plugin::path_rewrite::{get_path_rewrite_plugins, PathRewritePluginManager};
use crate::prelude::*;

pub use self::loader::PluginCategory;

pub mod connect_cost;
pub mod input_text;
mod loader;
pub mod oov;
pub mod path_rewrite;
pub mod dso;

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
        PluginError::Libloading { source: e, message: String::new() }
    }
}

/// Retrieves the path to the plugin shared object file from a plugin config
pub fn get_plugin_path(plugin_config: &Value, config: &Config) -> SudachiResult<PathBuf> {
    let obj = match plugin_config {
        Value::Object(v) => v,
        _ => {
            return Err(SudachiError::ConfigError(ConfigError::InvalidFormat(
                "plugin config must be an object".to_owned(),
            )));
        }
    };
    let lib = match obj.get("class") {
        Some(Value::String(v)) => v,
        _ => {
            return Err(SudachiError::ConfigError(ConfigError::InvalidFormat(
                "plugin config must have 'class' key to indicate plugin SO file".to_owned(),
            )));
        }
    };
    let lib_path = PathBuf::from(config.resolve_path(lib.clone()));
    Ok(lib_path)
}

pub(crate) struct Plugins {
    pub(crate) connect_cost: EditConnectionCostPluginManager,
    pub(crate) input_text: PluginContainer<dyn InputTextPlugin>,
    pub(crate) oov: OovProviderPluginManager,
    pub(crate) path_rewrite: PathRewritePluginManager,
}

impl Plugins {
    pub(crate) fn load(cfg: &Config, grammar: &Grammar) -> SudachiResult<Plugins> {
        let plugins = Plugins {
            connect_cost: get_edit_connection_cost_plugins(cfg, grammar)?,
            input_text: load_plugins_of(cfg, grammar)?,
            oov: get_oov_plugins(cfg, grammar)?,
            path_rewrite: get_path_rewrite_plugins(cfg, grammar)?,
        };
        Ok(plugins)
    }
}
