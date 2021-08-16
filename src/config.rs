use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use serde::Deserialize;
use serde_json::Value;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("IO Error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serde error: {0}")]
    SerdeError(#[from] serde_json::Error),

    #[error("Config file not found")]
    FileNotFound,
}

const DEFAULT_RESOURCE_DIR: &str = "./src/resources";
const DEFAULT_SETTING_FILE: &str = "sudachi.json";

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

#[derive(Debug, Default)]
pub struct Config {
    pub resource_dir: PathBuf,
    pub system_dict: Option<PathBuf>,
    pub user_dicts: Vec<PathBuf>,
    pub character_definition_file: Option<PathBuf>,

    pub connection_cost_plugins: Vec<Value>,
    pub input_text_plugins: Vec<Value>,
    pub oov_provider_plugins: Vec<Value>,
    pub path_rewrite_plugins: Vec<Value>,
}

impl Config {
    pub fn new(
        config_file: Option<PathBuf>,
        resource_dir: Option<PathBuf>,
        dictionary_path: Option<PathBuf>,
    ) -> Result<Self, ConfigError> {
        let config_file = match config_file {
            Some(v) => v,
            None => PathBuf::from(DEFAULT_RESOURCE_DIR).join(DEFAULT_SETTING_FILE),
        };
        let file = File::open(config_file)?;
        let reader = BufReader::new(file);
        let raw_config: RawConfig = serde_json::from_reader(reader)?;

        let resource_dir = resource_dir
            .or_else(|| raw_config.resourcePath.clone())
            .unwrap_or_else(|| PathBuf::from(DEFAULT_RESOURCE_DIR));
        let system_dict = dictionary_path
            .or_else(|| raw_config.systemDict.clone())
            .map(|p| Config::join_if_relative(&resource_dir, p));
        let user_dicts = raw_config
            .userDict
            .unwrap_or(Vec::new())
            .into_iter()
            .map(|p| Config::join_if_relative(&resource_dir, p))
            .collect();
        let character_definition_file = raw_config
            .characterDefinitionFile
            .map(|p| Config::join_if_relative(&resource_dir, p));

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

    pub fn complete_path(&self, file_path: PathBuf) -> PathBuf {
        Config::join_if_relative(&self.resource_dir, file_path)
    }
    fn join_if_relative(resource_dir: &PathBuf, file_path: PathBuf) -> PathBuf {
        if file_path.is_absolute() {
            file_path
        } else {
            resource_dir.join(file_path)
        }
    }
}
