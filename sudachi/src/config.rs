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

use serde::Deserialize;
use serde_json::Value;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use thiserror::Error;

const DEFAULT_RESOURCE_DIR: &str = "resources";
const DEFAULT_SETTING_FILE: &str = "sudachi.json";
const DEFAULT_CHAR_DEF_FILE: &str = "char.def";

/// Sudachi Error
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("IO Error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serde error: {0}")]
    SerdeError(#[from] serde_json::Error),

    #[error("Config file not found")]
    FileNotFound(String),

    #[error("Invalid format: {0}")]
    InvalidFormat(String),

    #[error("Argument {0} is missing")]
    MissingArgument(String),
}

/// Setting data loaded from config file
#[derive(Debug, Default)]
pub struct Config {
    pub resource_dir: PathBuf,
    pub system_dict: Option<PathBuf>,
    pub user_dicts: Vec<PathBuf>,
    pub character_definition_file: PathBuf,

    pub connection_cost_plugins: Vec<Value>,
    pub input_text_plugins: Vec<Value>,
    pub oov_provider_plugins: Vec<Value>,
    pub path_rewrite_plugins: Vec<Value>,
}

/// Struct corresponds with raw config json file.
/// You must use filed names defined here as json object key.
/// For plugins, refer to each plugin.
#[allow(non_snake_case)]
#[derive(Deserialize, Debug)]
pub struct RawConfig {
    resourcePath: Option<PathBuf>,
    systemDict: Option<PathBuf>,
    userDict: Option<Vec<PathBuf>>,
    characterDefinitionFile: Option<PathBuf>,

    connectionCostPlugin: Option<Vec<Value>>,
    inputTextPlugin: Option<Vec<Value>>,
    oovProviderPlugin: Option<Vec<Value>>,
    pathRewritePlugin: Option<Vec<Value>>,
}

impl Config {
    pub fn new(
        config_file: Option<PathBuf>,
        resource_dir: Option<PathBuf>,
        dictionary_path: Option<PathBuf>,
    ) -> Result<Self, ConfigError> {
        let src_root_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let default_resource_dir_path = src_root_path.join("..").join(DEFAULT_RESOURCE_DIR);

        // prioritize arg (cli option) > default
        let config_file = match config_file {
            Some(v) => v,
            None => default_resource_dir_path.join(DEFAULT_SETTING_FILE),
        };
        let file = File::open(config_file)?;
        let reader = BufReader::new(file);
        let raw_config: RawConfig = serde_json::from_reader(reader)?;

        // prioritize arg (cli option) > config file > default
        let resource_dir = resource_dir
            .or_else(|| raw_config.resourcePath.clone())
            .unwrap_or_else(|| default_resource_dir_path);

        // prioritize arg (cli option) > config file
        let system_dict = dictionary_path.or_else(|| {
            raw_config
                .systemDict
                .clone()
                .map(|p| Config::join_if_relative(&resource_dir, p))
        });
        let user_dicts = raw_config
            .userDict
            .unwrap_or(Vec::new())
            .into_iter()
            .map(|p| Config::join_if_relative(&resource_dir, p))
            .collect();
        let character_definition_file = Config::join_if_relative(
            &resource_dir,
            raw_config
                .characterDefinitionFile
                .unwrap_or(PathBuf::from(DEFAULT_CHAR_DEF_FILE)),
        );

        Ok(Config {
            resource_dir,
            system_dict,
            user_dicts,
            character_definition_file,

            connection_cost_plugins: raw_config.connectionCostPlugin.unwrap_or(Vec::new()),
            input_text_plugins: raw_config.inputTextPlugin.unwrap_or(Vec::new()),
            oov_provider_plugins: raw_config.oovProviderPlugin.unwrap_or(Vec::new()),
            path_rewrite_plugins: raw_config.pathRewritePlugin.unwrap_or(Vec::new()),
        })
    }

    /// Resolves given path to a path relative to resource_dir if its relative
    pub fn complete_path(&self, file_path: PathBuf) -> PathBuf {
        Config::join_if_relative(&self.resource_dir, file_path)
    }
    fn join_if_relative(resource_dir: &PathBuf, file_path: PathBuf) -> PathBuf {
        if file_path.is_absolute() {
            file_path
        } else {
            resource_dir.join(&file_path)
        }
    }
}
