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

pub mod connect_cost;
pub mod input_text;
pub mod oov;
pub mod path_rewrite;

use libloading::Error as LLError;
use serde_json::Value;
use std::path::PathBuf;
use thiserror::Error;

use crate::config::{Config, ConfigError};
use crate::prelude::*;

#[derive(Error, Debug)]
pub enum PluginError {
    #[error("IO Error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Libloading Error: {0}")]
    Libloading(#[from] LLError),

    #[error("Serde error: {0}")]
    SerdeError(#[from] serde_json::Error),

    #[error("Invalid data format: {0}")]
    InvalidDataFormat(String),
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
    Ok(config.complete_path(PathBuf::from(lib)))
}
