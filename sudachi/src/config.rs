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

    #[error("Failed to resolve relative path {0}, tried: {1:?}")]
    PathResolution(String, Vec<String>),
}

#[derive(Default, Debug, Clone)]
struct PathResolver {
    roots: Vec<PathBuf>,
}

impl PathResolver {
    fn with_capacity(capacity: usize) -> PathResolver {
        return PathResolver {
            roots: Vec::with_capacity(capacity),
        };
    }

    fn add<P: Into<PathBuf>>(&mut self, path: P) {
        self.roots.push(path.into())
    }

    fn contains<P: AsRef<Path>>(&self, path: P) -> bool {
        let query = path.as_ref();
        return self.roots.iter().find(|p| p.as_path() == query).is_some();
    }

    pub fn first_existing<P: AsRef<Path> + Clone>(&self, path: P) -> Option<PathBuf> {
        self.all_candidates(path).find(|p| p.exists())
    }

    pub fn resolution_failure<P: AsRef<Path> + Clone>(&self, path: P) -> ConfigError {
        let candidates = self
            .all_candidates(path.clone())
            .map(|p| p.to_string_lossy().into_owned())
            .collect();

        ConfigError::PathResolution(path.as_ref().to_string_lossy().into_owned(), candidates)
    }

    pub fn all_candidates<'a, P: AsRef<Path> + Clone + 'a>(
        &'a self,
        path: P,
    ) -> impl Iterator<Item = PathBuf> + 'a {
        self.roots.iter().map(move |root| root.join(path.clone()))
    }

    pub fn roots(&self) -> &[PathBuf] {
        return &self.roots;
    }
}

/// Setting data loaded from config file
#[derive(Debug, Default, Clone)]
pub struct Config {
    /// Paths will be resolved against these roots, until a file will be found
    resolver: PathResolver,
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
    /// Analogue to Java Implementation path Override
    path: Option<PathBuf>,
    /// User-passed resourcePath
    #[serde(skip)]
    resourcePath: Option<PathBuf>,
    /// User-passed root directory.
    /// Is also automatically set on from_file
    #[serde(skip)]
    rootDirectory: Option<PathBuf>,
    systemDict: Option<PathBuf>,
    userDict: Option<Vec<PathBuf>>,
    characterDefinitionFile: Option<PathBuf>,

    connectionCostPlugin: Option<Vec<Value>>,
    inputTextPlugin: Option<Vec<Value>>,
    oovProviderPlugin: Option<Vec<Value>>,
    pathRewritePlugin: Option<Vec<Value>>,
}

pub fn default_resource_dir() -> PathBuf {
    let mut src_root_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    if !src_root_path.pop() {
        src_root_path.push("..");
    }
    src_root_path.push(DEFAULT_RESOURCE_DIR);
    src_root_path
}

pub fn default_config_location() -> PathBuf {
    let mut resdir = default_resource_dir();
    resdir.push(DEFAULT_SETTING_FILE);
    resdir
}

impl ConfigBuilder {
    pub fn from_opt_file(config_file: Option<&Path>) -> Result<Self, ConfigError> {
        match config_file {
            None => {
                let default_config = default_config_location();
                Self::from_file(&default_config)
            }
            Some(cfg) => Self::from_file(cfg),
        }
    }

    pub fn from_file(config_file: &Path) -> Result<Self, ConfigError> {
        let file = File::open(config_file)?;
        let reader = BufReader::new(file);
        serde_json::from_reader(reader)
            .map_err(|e| e.into())
            .map(|cfg: ConfigBuilder| match config_file.parent() {
                Some(p) => cfg.root_directory(p),
                None => cfg,
            })
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

    pub fn root_directory(mut self, path: impl Into<PathBuf>) -> Self {
        self.rootDirectory = Some(path.into());
        self
    }

    pub fn build(self) -> Config {
        let default_resource_dir = default_resource_dir();
        let resource_dir = self.resourcePath.unwrap_or(default_resource_dir);

        let mut resolver = PathResolver::with_capacity(3);
        let mut add_path = |buf: PathBuf| {
            if !resolver.contains(&buf) {
                resolver.add(buf);
            }
        };
        self.path.map(&mut add_path);
        add_path(resource_dir);
        self.rootDirectory.map(&mut add_path);

        let character_definition_file = self
            .characterDefinitionFile
            .unwrap_or(PathBuf::from(DEFAULT_CHAR_DEF_FILE));

        Config {
            resolver,
            system_dict: self.systemDict,
            user_dicts: self.userDict.unwrap_or_else(|| Vec::new()),
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
        // prioritize arg (cli option) > default
        let raw_config = ConfigBuilder::from_opt_file(config_file.as_deref())?;

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
        let mut resolver = PathResolver::with_capacity(1);
        resolver.add(resource);
        cfg.resolver = resolver;
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

    pub fn resolve_paths(&self, mut path: String) -> Vec<String> {
        if path.starts_with("$exe") {
            path.replace_range(0..4, &CURRENT_EXE_DIR);

            let mut path2 = path.clone();
            path2.insert_str(CURRENT_EXE_DIR.len(), "/deps");
            return vec![path2, path];
        }

        if path.starts_with("$cfg/") || path.starts_with("$cfg\\") {
            let roots = self.resolver.roots();
            let mut result = Vec::with_capacity(roots.len());
            path.replace_range(0..5, "");
            for root in roots {
                let subpath = root.join(&path);
                result.push(subpath.to_string_lossy().into_owned());
            }
            return result;
        }

        vec![path]
    }

    /// Resolves a possibly relative path with regards to all possible anchors:
    /// 1. Absolute paths stay as they are
    /// 2. Paths are resolved wrt to anchors, returning the first existing one
    /// 3. Path are checked wrt to CWD
    /// 4. If all fail, return an error with all candidate paths listed
    pub fn complete_path<P: AsRef<Path> + Into<PathBuf>>(
        &self,
        file_path: P,
    ) -> Result<PathBuf, ConfigError> {
        let pref = file_path.as_ref();
        // 1. absolute paths are not normalized
        if pref.is_absolute() {
            return Ok(file_path.into());
        }

        // 2. try to resolve paths wrt anchors
        if let Some(p) = self.resolver.first_existing(pref) {
            return Ok(p);
        }

        // 3. try to resolve path wrt CWD
        if pref.exists() {
            return Ok(file_path.into());
        }

        // Report an error
        return Err(self.resolver.resolution_failure(&file_path));
    }

    pub fn resolved_system_dict(&self) -> Result<PathBuf, ConfigError> {
        match self.system_dict.as_ref() {
            Some(p) => self.complete_path(p),
            None => Err(ConfigError::MissingArgument("systemDict".to_owned())),
        }
    }

    pub fn resolved_user_dicts(&self) -> Result<Vec<PathBuf>, ConfigError> {
        self.user_dicts
            .iter()
            .map(|p| self.complete_path(p))
            .collect()
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
    use super::*;
    use crate::prelude::SudachiResult;

    use super::CURRENT_EXE_DIR;

    #[test]
    fn resolve_exe() -> SudachiResult<()> {
        let cfg = Config::new(None, None, None)?;
        let npath = cfg.resolve_paths("$exe/data".to_owned());
        let exe_dir: &str = &CURRENT_EXE_DIR;
        assert_eq!(npath.len(), 2);
        assert!(npath[0].starts_with(exe_dir));
        Ok(())
    }

    #[test]
    fn resolve_cfg() -> SudachiResult<()> {
        let cfg = Config::new(None, None, None)?;
        let npath = cfg.resolve_paths("$cfg/data".to_owned());
        let def = default_resource_dir();
        let path_dir: &str = def.to_str().unwrap();
        assert_eq!(1, npath.len());
        assert!(npath[0].starts_with(path_dir));
        Ok(())
    }
}
