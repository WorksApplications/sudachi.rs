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

use std::env::current_exe;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use lazy_static::lazy_static;
use serde::Deserialize;
use serde_json::Value;
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
#[derive(Debug, Default, Clone)]
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
pub struct ConfigBuilder {
    resourcePath: Option<PathBuf>,
    systemDict: Option<PathBuf>,
    userDict: Option<Vec<PathBuf>>,
    characterDefinitionFile: Option<PathBuf>,

    connectionCostPlugin: Option<Vec<Value>>,
    inputTextPlugin: Option<Vec<Value>>,
    oovProviderPlugin: Option<Vec<Value>>,
    pathRewritePlugin: Option<Vec<Value>>,
}

impl ConfigBuilder {
    pub fn from_file(config_file: &Path) -> Result<Self, ConfigError> {
        let file = File::open(config_file)?;
        let reader = BufReader::new(file);
        serde_json::from_reader(reader).map_err(|e| e.into())
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, ConfigError> {
        serde_json::from_slice(data).map_err(|e| e.into())
    }

    pub fn empty() -> Self {
        serde_json::from_slice(b"{}").unwrap()
    }

    pub fn system_dict(mut self, dict: impl Into<PathBuf>) -> Self {
        self.systemDict = Some(dict.into());
        self
    }

    pub fn user_dict(mut self, dict: impl Into<PathBuf>) -> Self {
        let dicts = match self.userDict.as_mut() {
            None => {
                self.userDict = Some(Default::default());
                self.userDict.as_mut().unwrap()
            }
            Some(dicts) => dicts,
        };
        dicts.push(dict.into());
        self
    }

    pub fn resource_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.resourcePath = Some(path.into());
        self
    }

    pub fn build(self) -> Config {
        let src_root_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let default_resource_dir_path = src_root_path.join("..").join(DEFAULT_RESOURCE_DIR);

        let resource_dir = self.resourcePath.unwrap_or(default_resource_dir_path);

        let system_dict = self
            .systemDict
            .clone()
            .map(|p| Config::join_if_relative(&resource_dir, p));

        let user_dicts = self
            .userDict
            .unwrap_or(Vec::new())
            .into_iter()
            .map(|p| Config::join_if_relative(&resource_dir, p))
            .collect();

        let character_definition_file = Config::join_if_relative(
            &resource_dir,
            self.characterDefinitionFile
                .unwrap_or(PathBuf::from(DEFAULT_CHAR_DEF_FILE)),
        );

        Config {
            resource_dir,
            system_dict,
            user_dicts,
            character_definition_file,

            connection_cost_plugins: self.connectionCostPlugin.unwrap_or(Vec::new()),
            input_text_plugins: self.inputTextPlugin.unwrap_or(Vec::new()),
            oov_provider_plugins: self.oovProviderPlugin.unwrap_or(Vec::new()),
            path_rewrite_plugins: self.pathRewritePlugin.unwrap_or(Vec::new()),
        }
    }
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

        let raw_config = ConfigBuilder::from_file(&config_file)?;

        // prioritize arg (cli option) > config file
        let raw_config = match resource_dir {
            None => raw_config,
            Some(p) => raw_config.resource_path(p),
        };

        // prioritize arg (cli option) > config file
        let raw_config = match dictionary_path {
            None => raw_config,
            Some(p) => raw_config.system_dict(p),
        };

        Ok(raw_config.build())
    }

    /// Creates a minimal config with the provided resource directory
    pub fn minimal_at(resource_dir: impl Into<PathBuf>) -> Config {
        let mut cfg = Config::default();
        let resource = resource_dir.into();
        cfg.character_definition_file = resource.join(DEFAULT_CHAR_DEF_FILE);
        cfg.resource_dir = resource;
        cfg.oov_provider_plugins = vec![serde_json::json!(
            { "class" : "com.worksap.nlp.sudachi.SimpleOovPlugin",
              "oovPOS" : [ "名詞", "普通名詞", "一般", "*", "*", "*" ],
              "leftId" : 0,
              "rightId" : 0,
              "cost" : 30000 }
        )];
        cfg
    }

    /// Sets the system dictionary to the provided path
    pub fn with_system_dic(mut self, system: impl Into<PathBuf>) -> Config {
        self.system_dict = Some(system.into());
        self
    }

    /// Resolve variables in path.
    /// Starting $exe is replaced with a directory of the current executable
    /// Starting $cfg is replaced with the resource directory
    ///
    /// Takes the input path as String, by value, because it will be modified.
    pub fn resolve_path(&self, mut path: String) -> String {
        if path.starts_with("$exe") {
            path.replace_range(0..4, &CURRENT_EXE_DIR);
        }

        if path.starts_with("$cfg") {
            let cfg_path = self.resource_dir.to_str().unwrap();
            path.replace_range(0..4, cfg_path);
        }

        path
    }

    pub fn resolve_plugin_paths(&self, mut path: String) -> Vec<String> {
        if path.starts_with("$exe") {
            path.replace_range(0..4, &CURRENT_EXE_DIR);

            let mut path2 = path.clone();
            path2.insert_str(CURRENT_EXE_DIR.len(), "/deps");
            return vec![path2, path];
        }

        if path.starts_with("$cfg") {
            let cfg_path = self.resource_dir.to_str().unwrap();
            path.replace_range(0..4, cfg_path);
        }

        vec![path]
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

fn current_exe_dir() -> String {
    let exe = current_exe().unwrap_or_else(|e| panic!("Current exe is not available {:?}", e));

    let parent = exe
        .parent()
        .unwrap_or_else(|| panic!("Path to executable must have a parent"));

    parent.to_str().map(|s| s.to_owned()).unwrap_or_else(|| {
        panic!("placing Sudachi in directories with non-utf paths is not supported")
    })
}

lazy_static! {
    static ref CURRENT_EXE_DIR: String = current_exe_dir();
}

#[cfg(test)]
mod tests {
    use crate::config::Config;
    use crate::prelude::SudachiResult;

    use super::CURRENT_EXE_DIR;

    #[test]
    fn resolve_exe() -> SudachiResult<()> {
        let cfg = Config::new(None, None, None)?;
        let npath = cfg.resolve_path("$exe/data".to_owned());
        let exe_dir: &str = &CURRENT_EXE_DIR;
        assert!(npath.starts_with(exe_dir));
        Ok(())
    }

    #[test]
    fn resolve_cfg() -> SudachiResult<()> {
        let cfg = Config::new(None, None, None)?;
        let npath = cfg.resolve_path("$cfg/data".to_owned());
        let path_dir: &str = cfg.resource_dir.to_str().unwrap();
        assert!(npath.starts_with(path_dir));
        Ok(())
    }
}
