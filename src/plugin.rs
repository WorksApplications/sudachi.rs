pub mod connect_cost;
pub mod input_text;
pub mod oov;
pub mod path_rewrite;

use libloading::Error as LLError;
use serde_json::Value;
use std::path::Path;
use thiserror::Error;

use crate::config::ConfigError;
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

pub fn get_plugin_path(plugin_config: &Value) -> SudachiResult<&Path> {
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
    Ok(Path::new(lib))
}
