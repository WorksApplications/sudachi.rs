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
use serde_json::Value;
use std::path::PathBuf;
use thiserror::Error;

use crate::config::{Config, ConfigError};
use crate::prelude::*;
use std::ffi::OsStr;
use crate::config::ConfigError::FileNotFound;
use thiserror::private::PathAsDisplay;

pub mod connect_cost;
pub mod input_text;
pub mod oov;
pub mod path_rewrite;

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

// All utilities for OsStr are very bad :\
fn starts_with_lib(data: &OsStr) -> bool {
    if data.len() < 3 {
        false
    } else {
        let lib_prefix: &OsStr = OsStr::new("lib");
        let lic_prefix: &OsStr = OsStr::new("lic");
        return lib_prefix.le(data) && lic_prefix.ge(data);
    }
}

fn fix_lib_extension(path: &PathBuf) -> PathBuf {
    let new_name = match path.file_name() {
        Some(name) =>
            if starts_with_lib(name) {
                name.to_str().map(|n| OsStr::new(&n[3..])).unwrap_or_else(|| name)
            } else { name }
        None => path.as_os_str()
    };
    let extension =
        if cfg!(target_os = "windows") {
            OsStr::new("dll")
        } else if cfg!(target_os = "linux") {
            OsStr::new("so")
        } else if cfg!(target_os = "macos") {
            OsStr::new("dylib")
        } else { panic!("Unsupported target! We support only Windows, Linux or MacOS") };
    path.with_file_name(new_name).with_extension(extension)
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
    let lib_path = config.complete_path(PathBuf::from(lib));
    if lib_path.exists() {
        return Ok(lib_path);
    }
    let fixed_path = fix_lib_extension(&lib_path);
    if fixed_path.exists() {
        return Ok(fixed_path);
    }
    Err(SudachiError::ConfigError(FileNotFound(format!("Failed to find library, tried: {} and {}", lib_path.as_display(), fixed_path.as_display()))))
}
