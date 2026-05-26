/*
 * Copyright (c) 2021-2026 Works Applications Co., Ltd.
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

use std::convert::TryFrom;
use std::env::current_exe;
use std::fs::File;
use std::io::{BufRead, BufReader, Cursor};
use std::path::{Path, PathBuf};

use crate::dic::subset::InfoSubset;
use crate::error::SudachiError;
use lazy_static::lazy_static;
use serde::Deserialize;
use serde_json::Value;
use thiserror::Error;

const DEFAULT_SETTING_FILE: &str = "sudachi.json";
const DEFAULT_SETTING_BYTES: &[u8] = include_bytes!("../../resources/sudachi.json");
pub(crate) const DEFAULT_CHAR_DEF_FILE: &str = "char.def";
pub(crate) const DEFAULT_CHAR_DEF_BYTES: &[u8] = include_bytes!("../../resources/char.def");
pub(crate) const DEFAULT_UNK_DEF_FILE: &str = "unk.def";
const DEFAULT_UNK_DEF_BYTES: &[u8] = include_bytes!("../../resources/unk.def");
pub(crate) const DEFAULT_REWRITE_DEF_FILE: &str = "rewrite.def";
const DEFAULT_REWRITE_DEF_BYTES: &[u8] = include_bytes!("../../resources/rewrite.def");

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

    #[error("{0} is only available as an embedded resource")]
    EmbeddedResourcePath(String),

    #[error("Failed to resolve relative path {0}, tried: {1:?}")]
    PathResolution(String, Vec<String>),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum EmbeddedResource {
    Config,
    CharDef,
    UnkDef,
    RewriteDef,
}

impl EmbeddedResource {
    fn name(self) -> &'static str {
        match self {
            EmbeddedResource::Config => DEFAULT_SETTING_FILE,
            EmbeddedResource::CharDef => DEFAULT_CHAR_DEF_FILE,
            EmbeddedResource::UnkDef => DEFAULT_UNK_DEF_FILE,
            EmbeddedResource::RewriteDef => DEFAULT_REWRITE_DEF_FILE,
        }
    }

    fn bytes(self) -> &'static [u8] {
        match self {
            EmbeddedResource::Config => DEFAULT_SETTING_BYTES,
            EmbeddedResource::CharDef => DEFAULT_CHAR_DEF_BYTES,
            EmbeddedResource::UnkDef => DEFAULT_UNK_DEF_BYTES,
            EmbeddedResource::RewriteDef => DEFAULT_REWRITE_DEF_BYTES,
        }
    }

    fn from_path<P: AsRef<Path>>(path: P) -> Option<Self> {
        let path = path.as_ref();
        if path.is_absolute() || path.components().count() != 1 {
            return None;
        }
        match path.to_str()? {
            DEFAULT_SETTING_FILE => Some(EmbeddedResource::Config),
            DEFAULT_CHAR_DEF_FILE => Some(EmbeddedResource::CharDef),
            DEFAULT_UNK_DEF_FILE => Some(EmbeddedResource::UnkDef),
            DEFAULT_REWRITE_DEF_FILE => Some(EmbeddedResource::RewriteDef),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ResolvedResource {
    Path(PathBuf),
    Embedded(EmbeddedResource),
}

impl ResolvedResource {
    pub fn read_bytes(self) -> Result<Vec<u8>, ConfigError> {
        match self {
            ResolvedResource::Path(path) => std::fs::read(path).map_err(ConfigError::from),
            ResolvedResource::Embedded(resource) => Ok(resource.bytes().to_vec()),
        }
    }

    pub fn reader(self) -> Result<Box<dyn BufRead>, ConfigError> {
        match self {
            ResolvedResource::Path(path) => {
                let file = File::open(path)?;
                Ok(Box::new(BufReader::new(file)))
            }
            ResolvedResource::Embedded(resource) => {
                Ok(Box::new(BufReader::new(Cursor::new(resource.bytes()))))
            }
        }
    }

    pub fn into_path(self) -> Result<PathBuf, ConfigError> {
        match self {
            ResolvedResource::Path(path) => Ok(path),
            ResolvedResource::Embedded(resource) => Err(ConfigError::EmbeddedResourcePath(
                resource.name().to_owned(),
            )),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ResolverRoot {
    Filesystem(PathBuf),
    Embedded,
}

#[derive(Default, Debug, Clone)]
struct PathResolver {
    roots: Vec<ResolverRoot>,
}

impl PathResolver {
    fn with_capacity(capacity: usize) -> PathResolver {
        PathResolver {
            roots: Vec::with_capacity(capacity),
        }
    }

    fn add<P: Into<PathBuf>>(&mut self, path: P) {
        self.roots.push(ResolverRoot::Filesystem(path.into()))
    }

    fn add_embedded(&mut self) {
        self.roots.push(ResolverRoot::Embedded)
    }

    fn contains<P: AsRef<Path>>(&self, path: P) -> bool {
        let query = path.as_ref();
        self.roots.iter().any(|p| match p {
            ResolverRoot::Filesystem(root) => root.as_path() == query,
            ResolverRoot::Embedded => false,
        })
    }

    fn contains_embedded(&self) -> bool {
        self.roots.contains(&ResolverRoot::Embedded)
    }

    pub fn first_existing<P: AsRef<Path> + Clone>(&self, path: P) -> Option<ResolvedResource> {
        self.roots.iter().find_map(|root| match root {
            ResolverRoot::Filesystem(base) => {
                let candidate = base.join(path.clone());
                candidate
                    .exists()
                    .then_some(ResolvedResource::Path(candidate))
            }
            ResolverRoot::Embedded => {
                EmbeddedResource::from_path(path.clone()).map(ResolvedResource::Embedded)
            }
        })
    }

    pub fn resolution_failure<P: AsRef<Path> + Clone>(&self, path: P) -> ConfigError {
        let candidates = self.all_candidates(path.clone()).collect();

        ConfigError::PathResolution(path.as_ref().to_string_lossy().into_owned(), candidates)
    }

    pub fn all_candidates<'a, P: AsRef<Path> + Clone + 'a>(
        &'a self,
        path: P,
    ) -> impl Iterator<Item = String> + 'a {
        self.roots.iter().map(move |root| match root {
            ResolverRoot::Filesystem(base) => {
                base.join(path.clone()).to_string_lossy().into_owned()
            }
            ResolverRoot::Embedded => format!("<embedded>/{}", path.as_ref().display()),
        })
    }

    pub fn filesystem_roots(&self) -> impl Iterator<Item = &PathBuf> {
        self.roots.iter().filter_map(|root| match root {
            ResolverRoot::Filesystem(path) => Some(path),
            ResolverRoot::Embedded => None,
        })
    }
}

#[derive(Deserialize, Clone, Copy, Debug, Eq, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceProjection {
    #[default]
    Surface,
    Normalized,
    Reading,
    Dictionary,
    DictionaryAndSurface,
    NormalizedAndSurface,
    NormalizedNouns,
}

impl SurfaceProjection {
    /// Return required InfoSubset for the current projection type
    pub fn required_subset(&self) -> InfoSubset {
        match *self {
            SurfaceProjection::Surface => InfoSubset::empty(),
            SurfaceProjection::Normalized => InfoSubset::NORMALIZED_FORM,
            SurfaceProjection::Reading => InfoSubset::READING_FORM,
            SurfaceProjection::Dictionary => InfoSubset::DICTIONARY_FORM,
            SurfaceProjection::DictionaryAndSurface => InfoSubset::DICTIONARY_FORM,
            SurfaceProjection::NormalizedAndSurface => InfoSubset::NORMALIZED_FORM,
            SurfaceProjection::NormalizedNouns => InfoSubset::NORMALIZED_FORM,
        }
    }
}

impl TryFrom<&str> for SurfaceProjection {
    type Error = SudachiError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "surface" => Ok(SurfaceProjection::Surface),
            "normalized" => Ok(SurfaceProjection::Normalized),
            "reading" => Ok(SurfaceProjection::Reading),
            "dictionary" => Ok(SurfaceProjection::Dictionary),
            "dictionary_and_surface" => Ok(SurfaceProjection::DictionaryAndSurface),
            "normalized_and_surface" => Ok(SurfaceProjection::NormalizedAndSurface),
            "normalized_nouns" => Ok(SurfaceProjection::NormalizedNouns),
            _ => Err(ConfigError::InvalidFormat(format!("unknown projection: {value}")).into()),
        }
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
    // this option is Python-only and is ignored in Rust APIs
    pub projection: SurfaceProjection,
}

/// Struct corresponds with raw config json file.
/// You must use filed names defined here as json object key.
/// For plugins, refer to each plugin.
#[allow(non_snake_case)]
#[derive(Deserialize, Debug, Clone)]
pub struct ConfigBuilder {
    /// Analogue to Java Implementation path Override.
    path: Option<PathBuf>,
    /// User-passed resourcePath.
    #[serde(skip)]
    resourcePath: Option<PathBuf>,
    /// User-passed root directory.
    /// Is also automatically set on from_file.
    #[serde(skip)]
    rootDirectory: Option<PathBuf>,
    /// Use embedded resource data if true.
    #[serde(skip)]
    embedded_resources: bool,
    #[serde(alias = "system")]
    systemDict: Option<PathBuf>,
    #[serde(alias = "user")]
    userDict: Option<Vec<PathBuf>>,
    characterDefinitionFile: Option<PathBuf>,
    connectionCostPlugin: Option<Vec<Value>>,
    inputTextPlugin: Option<Vec<Value>>,
    oovProviderPlugin: Option<Vec<Value>>,
    pathRewritePlugin: Option<Vec<Value>>,
    projection: Option<SurfaceProjection>,
}

macro_rules! merge_cfg_value {
    ($base: ident, $o: ident, $name: tt) => {
        $base.$name = $base.$name.or_else(|| $o.$name.clone())
    };
}

impl ConfigBuilder {
    pub fn from_opt_file(config_file: Option<&Path>) -> Result<Self, ConfigError> {
        match config_file {
            None => Self::from_embedded(),
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

    pub fn from_embedded() -> Result<Self, ConfigError> {
        Self::from_bytes(DEFAULT_SETTING_BYTES)
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

    pub fn embedded_resources(mut self, enabled: bool) -> Self {
        self.embedded_resources = enabled;
        self
    }

    pub fn build(self) -> Config {
        let mut resolver = PathResolver::with_capacity(3);
        let mut add_path = |buf: PathBuf| {
            if !resolver.contains(&buf) {
                resolver.add(buf);
            }
        };
        self.path.map(&mut add_path);
        self.resourcePath.map(&mut add_path);
        self.rootDirectory.map(&mut add_path);
        if self.embedded_resources && !resolver.contains_embedded() {
            resolver.add_embedded();
        }

        let character_definition_file = self
            .characterDefinitionFile
            .unwrap_or(PathBuf::from(DEFAULT_CHAR_DEF_FILE));

        Config {
            resolver,
            system_dict: self.systemDict,
            user_dicts: self.userDict.unwrap_or_default(),
            character_definition_file,

            connection_cost_plugins: self.connectionCostPlugin.unwrap_or_default(),
            input_text_plugins: self.inputTextPlugin.unwrap_or_default(),
            oov_provider_plugins: self.oovProviderPlugin.unwrap_or_default(),
            path_rewrite_plugins: self.pathRewritePlugin.unwrap_or_default(),
            projection: self.projection.unwrap_or(SurfaceProjection::Surface),
        }
    }

    pub fn fallback(mut self, other: &ConfigBuilder) -> ConfigBuilder {
        merge_cfg_value!(self, other, path);
        merge_cfg_value!(self, other, resourcePath);
        merge_cfg_value!(self, other, rootDirectory);
        merge_cfg_value!(self, other, systemDict);
        merge_cfg_value!(self, other, userDict);
        merge_cfg_value!(self, other, characterDefinitionFile);
        merge_cfg_value!(self, other, connectionCostPlugin);
        merge_cfg_value!(self, other, inputTextPlugin);
        merge_cfg_value!(self, other, oovProviderPlugin);
        merge_cfg_value!(self, other, pathRewritePlugin);
        merge_cfg_value!(self, other, projection);
        self.embedded_resources |= other.embedded_resources;
        self
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
            Some(p) => raw_config.resource_path(p),
            None => raw_config,
        };

        // prioritize arg (cli option) > config file
        let raw_config = match dictionary_path {
            Some(p) => raw_config.system_dict(p),
            None => raw_config,
        };

        Ok(raw_config.build())
    }

    pub fn new_embedded() -> Result<Self, ConfigError> {
        let raw_config = ConfigBuilder::from_embedded()?;

        Ok(raw_config.build())
    }

    /// Creates a minimal config with the provided resource directory
    pub fn minimal_at(resource_dir: impl Into<PathBuf>) -> Config {
        let mut cfg = Config::default();
        let resource = resource_dir.into();
        cfg.character_definition_file = PathBuf::from(DEFAULT_CHAR_DEF_FILE);
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
            let mut result = Vec::new();
            path.replace_range(0..5, "");
            for root in self.resolver.filesystem_roots() {
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
        self.resolve_resource(file_path)?.into_path()
    }

    pub(crate) fn resolve_resource<P: AsRef<Path> + Into<PathBuf>>(
        &self,
        file_path: P,
    ) -> Result<ResolvedResource, ConfigError> {
        let pref = file_path.as_ref();
        // 1. absolute paths are not normalized
        if pref.is_absolute() {
            return Ok(ResolvedResource::Path(file_path.into()));
        }

        // 2. try to resolve paths wrt anchors
        if let Some(p) = self.resolver.first_existing(pref) {
            return Ok(p);
        }

        // 3. try to resolve path wrt CWD
        if pref.exists() {
            return Ok(ResolvedResource::Path(file_path.into()));
        }

        // Report an error
        Err(self.resolver.resolution_failure(&file_path))
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
        assert!(npath.is_empty());
        Ok(())
    }

    #[test]
    fn config_builder_fallback() {
        let mut cfg = ConfigBuilder::empty();
        cfg.path = Some("test".into());
        let cfg2 = ConfigBuilder::empty();
        let cfg2 = cfg2.fallback(&cfg);
        assert_eq!(cfg2.path, Some("test".into()));
    }

    #[test]
    fn embedded_resources_are_enabled_by_default() -> SudachiResult<()> {
        let cfg = ConfigBuilder::empty().build();
        let res = cfg.resolve_resource(DEFAULT_CHAR_DEF_FILE)?;
        assert!(matches!(
            res,
            ResolvedResource::Embedded(EmbeddedResource::CharDef)
        ));
        Ok(())
    }

    #[test]
    fn embedded_resources_can_be_disabled() {
        let cfg = ConfigBuilder::empty().embedded_resources(false).build();
        let err = cfg.resolve_resource(DEFAULT_CHAR_DEF_FILE).unwrap_err();
        assert!(matches!(err, ConfigError::PathResolution(_, _)));
    }

    #[test]
    fn embedded_resource_can_not_be_forced_into_path() {
        let cfg = ConfigBuilder::empty().build();
        let err = cfg.complete_path(DEFAULT_CHAR_DEF_FILE).unwrap_err();
        assert!(matches!(
            err,
            ConfigError::EmbeddedResourcePath(name) if name == DEFAULT_CHAR_DEF_FILE
        ));
    }

    #[test]
    fn surface_projection_tryfrom() {
        assert_eq!(
            SurfaceProjection::Surface,
            SurfaceProjection::try_from("surface").unwrap()
        );
    }
}
