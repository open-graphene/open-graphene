use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::error::{Result, SpecGenError};

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct GeneratorConfig {
    pub chain: ChainConfig,
    pub source: SourceConfig,
    pub output: OutputConfig,
    #[serde(default)]
    pub rpc_apis: Vec<RpcApiConfig>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct ChainConfig {
    pub id: String,
    pub public_key_prefix: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct SourceConfig {
    pub chain_repo: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct OutputConfig {
    pub dist: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct RpcApiConfig {
    pub name: String,
    #[serde(rename = "class")]
    pub api_class: String,
    pub discover_method: Option<String>,
    #[serde(default)]
    pub methods: Vec<String>,
}

pub fn load_config(path: impl AsRef<Path>) -> Result<GeneratorConfig> {
    let path = path.as_ref();
    let source = fs::read_to_string(path).map_err(|source| SpecGenError::ReadConfig {
        path: path.to_path_buf(),
        source,
    })?;

    toml::from_str(&source).map_err(|source| SpecGenError::ParseConfig {
        path: path.to_path_buf(),
        source,
    })
}
