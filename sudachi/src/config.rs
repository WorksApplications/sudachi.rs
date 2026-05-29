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
use serde::{Deserialize, Serialize};
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
pub struct PathResolver {
    roots: Vec<ResolverRoot>,
}

impl PathResolver {
    pub fn new() -> PathResolver {
        Self::default()
    }

    pub fn from_path<P: Into<PathBuf>>(path: P) -> Self {
        let mut resolver = Self::new();
        resolver.push_root(path);
        resolver
    }

    pub fn from_embedded() -> Self {
        let mut resolver = Self::new();
        resolver.push_embedded();
        resolver
    }

    pub fn push_root<P: Into<PathBuf>>(&mut self, path: P) {
        let path = path.into();
        if !self.contains(&path) {
            self.roots.push(ResolverRoot::Filesystem(path))
        }
    }

    pub fn prepend_root<P: Into<PathBuf>>(&mut self, path: P) {
        let path = path.into();
        self.roots.retain(|root| match root {
            ResolverRoot::Filesystem(existing) => existing != &path,
            ResolverRoot::Embedded => true,
        });
        self.roots.insert(0, ResolverRoot::Filesystem(path));
    }

    pub fn push_embedded(&mut self) {
        if !self.contains_embedded() {
            self.roots.push(ResolverRoot::Embedded)
        }
    }

    pub fn prepend_embedded(&mut self) {
        self.roots.retain(|root| root != &ResolverRoot::Embedded);
        self.roots.insert(0, ResolverRoot::Embedded);
    }

    pub fn append(&mut self, other: PathResolver) {
        for root in other.roots {
            match root {
                ResolverRoot::Filesystem(path) => self.push_root(path),
                ResolverRoot::Embedded => self.push_embedded(),
            }
        }
    }

    pub fn prepend(&mut self, other: PathResolver) {
        for root in other.roots.into_iter().rev() {
            match root {
                ResolverRoot::Filesystem(path) => self.prepend_root(path),
                ResolverRoot::Embedded => self.prepend_embedded(),
            }
        }
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

    fn first_existing<P: AsRef<Path> + Clone>(&self, path: P) -> Option<ResolvedResource> {
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

    fn resolution_failure<P: AsRef<Path> + Clone>(&self, path: P) -> ConfigError {
        let candidates = self.all_candidates(path.clone()).collect();

        ConfigError::PathResolution(path.as_ref().to_string_lossy().into_owned(), candidates)
    }

    fn all_candidates<'a, P: AsRef<Path> + Clone + 'a>(
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

    fn filesystem_roots(&self) -> impl Iterator<Item = &PathBuf> {
        self.roots.iter().filter_map(|root| match root {
            ResolverRoot::Filesystem(path) => Some(path),
            ResolverRoot::Embedded => None,
        })
    }
}

#[derive(Deserialize, Serialize, Clone, Copy, Debug, Eq, PartialEq, Default)]
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

/// Struct corresponds with raw config json file.
/// You must use filed names defined here as json object key.
/// For plugins, refer to each plugin.
#[allow(non_snake_case)]
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct RawConfig {
    /// Analogue to Java Implementation path Override.
    path: Option<PathBuf>,
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

#[derive(Debug, Clone, Default)]
/// Editable configuration source consisting of normalized config data and
/// explicit resource-resolution state.
pub struct ConfigBuilder {
    data: RawConfig,
    resolver: PathResolver,
}

macro_rules! merge_cfg_value {
    ($base: expr, $o: expr, $name: tt) => {
        $base.$name = $base.$name.or_else(|| $o.$name.clone())
    };
}

impl ConfigBuilder {
    /// Creates a builder from already-deserialized config data.
    ///
    /// If the raw config contains `path`, it is immediately appended to the
    /// resolver so runtime resolution state is explicit from construction time.
    fn from_data(data: RawConfig) -> Self {
        let mut resolver = PathResolver::new();
        if let Some(path) = data.path.clone() {
            resolver.push_root(path);
        }
        Self { data, resolver }
    }

    pub fn from_opt_file(config_file: Option<&Path>) -> Result<Self, ConfigError> {
        match config_file {
            None => Self::from_embedded(),
            Some(cfg) => Self::from_file(cfg),
        }
    }

    /// Loads config JSON from a file.
    ///
    /// The resulting builder contains the deserialized config data, appends the
    /// config's `path` field to the resolver if present, and then appends the
    /// parent directory of `config_file` as an additional filesystem root.
    pub fn from_file(config_file: &Path) -> Result<Self, ConfigError> {
        let file = File::open(config_file)?;
        let reader = BufReader::new(file);
        let data: RawConfig = serde_json::from_reader(reader).map_err(ConfigError::from)?;
        let mut cfg = Self::from_data(data);
        if let Some(parent) = config_file.parent() {
            cfg.resolver.push_root(parent);
        }
        Ok(cfg)
    }

    /// Loads config JSON from raw bytes.
    ///
    /// The resulting builder contains the deserialized config data and appends
    /// the config's `path` field to the resolver if present.
    pub fn from_bytes(data: &[u8]) -> Result<Self, ConfigError> {
        let data = serde_json::from_slice(data).map_err(ConfigError::from)?;
        Ok(Self::from_data(data))
    }

    /// Loads the bundled default config JSON.
    ///
    /// This only loads the embedded JSON contents; embedded resources
    /// themselves are not enabled unless `push_embedded()` is called.
    pub fn from_embedded() -> Result<Self, ConfigError> {
        Self::from_bytes(DEFAULT_SETTING_BYTES)
    }

    pub fn empty() -> Self {
        Self::default()
    }

    pub fn system_dict(mut self, dict: impl Into<PathBuf>) -> Self {
        self.data.systemDict = Some(dict.into());
        self
    }

    pub fn user_dict(mut self, dict: impl Into<PathBuf>) -> Self {
        let dicts = match self.data.userDict.as_mut() {
            None => {
                self.data.userDict = Some(Default::default());
                self.data.userDict.as_mut().unwrap()
            }
            Some(dicts) => dicts,
        };
        dicts.push(dict.into());
        self
    }

    pub fn character_definition_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.data.characterDefinitionFile = Some(path.into());
        self
    }

    pub fn projection(mut self, projection: SurfaceProjection) -> Self {
        self.data.projection = Some(projection);
        self
    }

    pub fn with_resolver(mut self, resolver: PathResolver) -> Self {
        self.resolver = resolver;
        self
    }

    pub fn append_resolver(mut self, other: PathResolver) -> Self {
        self.resolver.append(other);
        self
    }

    pub fn prepend_resolver(mut self, other: PathResolver) -> Self {
        self.resolver.prepend(other);
        self
    }

    pub fn push_resolver_root(mut self, path: impl Into<PathBuf>) -> Self {
        self.resolver.push_root(path);
        self
    }

    pub fn prepend_resolver_root(mut self, path: impl Into<PathBuf>) -> Self {
        self.resolver.prepend_root(path);
        self
    }

    pub fn push_embedded(mut self) -> Self {
        self.resolver.push_embedded();
        self
    }

    pub fn fallback_data(mut self, other: &ConfigBuilder) -> ConfigBuilder {
        merge_cfg_value!(self.data, other.data, path);
        merge_cfg_value!(self.data, other.data, systemDict);
        merge_cfg_value!(self.data, other.data, userDict);
        merge_cfg_value!(self.data, other.data, characterDefinitionFile);
        merge_cfg_value!(self.data, other.data, connectionCostPlugin);
        merge_cfg_value!(self.data, other.data, inputTextPlugin);
        merge_cfg_value!(self.data, other.data, oovProviderPlugin);
        merge_cfg_value!(self.data, other.data, pathRewritePlugin);
        merge_cfg_value!(self.data, other.data, projection);
        self
    }

    pub fn raw_config(&self) -> &RawConfig {
        &self.data
    }

    pub fn into_raw_config(self) -> RawConfig {
        self.data
    }

    pub fn build(self) -> Config {
        let character_definition_file = self
            .data
            .characterDefinitionFile
            .unwrap_or(PathBuf::from(DEFAULT_CHAR_DEF_FILE));

        Config {
            resolver: self.resolver,
            system_dict: self.data.systemDict,
            user_dicts: self.data.userDict.unwrap_or_default(),
            character_definition_file,

            connection_cost_plugins: self.data.connectionCostPlugin.unwrap_or_default(),
            input_text_plugins: self.data.inputTextPlugin.unwrap_or_default(),
            oov_provider_plugins: self.data.oovProviderPlugin.unwrap_or_default(),
            path_rewrite_plugins: self.data.pathRewritePlugin.unwrap_or_default(),
            projection: self.data.projection.unwrap_or(SurfaceProjection::Surface),
        }
    }
}

/// Setting data loaded from config file
#[derive(Debug, Default, Clone)]
pub struct Config {
    /// Paths will be resolved against these roots, until a file will be found
    pub resolver: PathResolver,
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

impl Config {
    /// Creates a config from optional config, resource, and dictionary paths.
    ///
    /// Resolution precedence is:
    /// 1. `resource_dir` (if given).
    /// 2. The `path` field in the `config_file` (if set).
    /// 3. The parent directory of the `config_file` (if given).
    /// 4. The default (embedded) resources.
    ///
    /// When `config_file` is `None`, the embedded default `sudachi.json` is used as config
    /// data.
    pub fn new(
        config_file: Option<PathBuf>,
        resource_dir: Option<PathBuf>,
        dictionary_path: Option<PathBuf>,
    ) -> Result<Self, ConfigError> {
        // prioritize arg (cli option) > default
        let raw_config = ConfigBuilder::from_opt_file(config_file.as_deref())?;

        // prioritize arg (cli option) > config file
        let raw_config = match resource_dir {
            Some(p) => raw_config.prepend_resolver_root(p),
            None => raw_config,
        }
        .push_embedded();

        // prioritize arg (cli option) > config file
        let raw_config = match dictionary_path {
            Some(p) => raw_config.system_dict(p),
            None => raw_config,
        };

        Ok(raw_config.build())
    }

    pub fn empty() -> Self {
        Self::default()
    }

    pub fn new_embedded() -> Result<Self, ConfigError> {
        let raw_config = ConfigBuilder::from_embedded()?.push_embedded();
        Ok(raw_config.build())
    }

    /// Creates a minimal config with the provided path resolver
    pub fn minimal_at(resolver: PathResolver) -> Config {
        let mut cfg = ConfigBuilder::empty().append_resolver(resolver).build();
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
        let cfg = ConfigBuilder::from_bytes(br#"{ "path": "test" }"#).unwrap();
        let cfg2 = ConfigBuilder::empty();
        let cfg2 = cfg2.fallback_data(&cfg);
        assert_eq!(cfg2.raw_config().path, Some("test".into()));
    }

    #[test]
    fn embedded_resources_can_be_enabled_explicitly() -> SudachiResult<()> {
        let cfg = ConfigBuilder::empty().push_embedded().build();
        let res = cfg.resolve_resource(DEFAULT_CHAR_DEF_FILE)?;
        assert!(matches!(
            res,
            ResolvedResource::Embedded(EmbeddedResource::CharDef)
        ));
        Ok(())
    }

    #[test]
    fn embedded_resources_are_disabled_by_default() {
        let cfg = ConfigBuilder::empty().build();
        let err = cfg.resolve_resource(DEFAULT_CHAR_DEF_FILE).unwrap_err();
        assert!(matches!(err, ConfigError::PathResolution(_, _)));
    }

    #[test]
    fn embedded_resource_can_not_be_forced_into_path() {
        let cfg = ConfigBuilder::empty().push_embedded().build();
        let err = cfg.complete_path(DEFAULT_CHAR_DEF_FILE).unwrap_err();
        assert!(matches!(
            err,
            ConfigError::EmbeddedResourcePath(name) if name == DEFAULT_CHAR_DEF_FILE
        ));
    }

    #[test]
    fn from_file_sets_path_before_parent() -> SudachiResult<()> {
        let cfg_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/resources/sudachi.json");
        let cfg = ConfigBuilder::from_file(&cfg_path)?;
        let roots: Vec<_> = cfg.raw_config().path.iter().cloned().collect();
        assert_eq!(roots, vec![PathBuf::from("tests/resources/")]);
        let npath = cfg.build().resolve_paths("$cfg/data".to_owned());
        assert_eq!(npath[0], "tests/resources/data");
        assert!(npath[1].ends_with("sudachi/tests/resources/data"));
        Ok(())
    }

    #[test]
    fn prepend_resolver_root_takes_priority() {
        let cfg = ConfigBuilder::from_bytes(br#"{ "path": "config-root" }"#)
            .unwrap()
            .prepend_resolver_root("resource-root");
        let roots: Vec<_> = cfg.resolver.filesystem_roots().cloned().collect();
        assert_eq!(
            roots,
            vec![PathBuf::from("resource-root"), PathBuf::from("config-root")]
        );
    }

    #[test]
    fn surface_projection_tryfrom() {
        assert_eq!(
            SurfaceProjection::Surface,
            SurfaceProjection::try_from("surface").unwrap()
        );
    }

    #[test]
    fn config_builder_sets_character_definition_file() {
        let cfg = ConfigBuilder::empty()
            .character_definition_file("custom.def")
            .build();
        assert_eq!(cfg.character_definition_file, PathBuf::from("custom.def"));
    }

    #[test]
    fn config_builder_sets_projection() {
        let cfg = ConfigBuilder::empty()
            .projection(SurfaceProjection::Reading)
            .build();
        assert_eq!(cfg.projection, SurfaceProjection::Reading);
    }
}
