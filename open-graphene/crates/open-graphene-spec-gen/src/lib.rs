//! Open Graphene protocol specification generator.
//!
//! This crate reads project-local `open-graphene.toml` files and emits
//! `open_graphene_json_schema::Protocol` JSON documents. Source-code extraction
//! will be layered on top of this config/output pipeline.

use std::fs;
use std::path::{Path, PathBuf};

use open_graphene_json_schema::{ChainDef, Protocol, RpcApiDef};
use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SpecGenError {
    #[error("failed to read config {path}: {source}")]
    ReadConfig {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse TOML config {path}: {source}")]
    ParseConfig {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
    #[error("output path in config {config_path} must have a parent directory: {dist}")]
    OutputPathHasNoParent { config_path: PathBuf, dist: String },
    #[error("failed to create output directory {path}: {source}")]
    CreateOutputDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to serialize protocol JSON for {config_path}: {source}")]
    SerializeProtocol {
        config_path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("failed to write protocol JSON {path}: {source}")]
    WriteOutput {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerateResult {
    pub output_path: PathBuf,
    pub rpc_api_count: usize,
    pub selected_method_count: usize,
}

pub fn load_config(path: impl AsRef<Path>) -> Result<GeneratorConfig, SpecGenError> {
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

pub fn build_protocol(config: &GeneratorConfig) -> Protocol {
    Protocol {
        schema_version: 1,
        chain: ChainDef {
            id: config.chain.id.clone(),
            public_key_prefix: config.chain.public_key_prefix.clone(),
        },
        structs: vec![],
        enums: vec![],
        static_variants: vec![],
        operations: vec![],
        object_types: vec![],
        rpc_apis: config
            .rpc_apis
            .iter()
            .map(|api| RpcApiDef {
                name: api.name.clone(),
                api_class: api.api_class.clone(),
                access_name: Some(api.api_class.clone()),
                discover_method: api.discover_method.clone(),
                required: true,
                source: None,
                support: None,
            })
            .collect(),
        rpc_methods: vec![],
        strict_mode: None,
    }
}

pub fn generate_from_config(path: impl AsRef<Path>) -> Result<GenerateResult, SpecGenError> {
    let config_path = path.as_ref();
    let config = load_config(config_path)?;
    let protocol = build_protocol(&config);
    let output_path = resolve_dist_path(config_path, &config.output.dist)?;

    let output_dir = output_path
        .parent()
        .ok_or_else(|| SpecGenError::OutputPathHasNoParent {
            config_path: config_path.to_path_buf(),
            dist: config.output.dist.clone(),
        })?;
    fs::create_dir_all(output_dir).map_err(|source| SpecGenError::CreateOutputDir {
        path: output_dir.to_path_buf(),
        source,
    })?;

    let json = serde_json::to_string_pretty(&protocol).map_err(|source| {
        SpecGenError::SerializeProtocol {
            config_path: config_path.to_path_buf(),
            source,
        }
    })?;
    fs::write(&output_path, format!("{json}\n")).map_err(|source| SpecGenError::WriteOutput {
        path: output_path.clone(),
        source,
    })?;

    Ok(GenerateResult {
        output_path,
        rpc_api_count: config.rpc_apis.len(),
        selected_method_count: config.rpc_apis.iter().map(|api| api.methods.len()).sum(),
    })
}

fn resolve_dist_path(config_path: &Path, dist: &str) -> Result<PathBuf, SpecGenError> {
    let dist_path = PathBuf::from(dist);
    if dist_path.is_absolute() {
        return Ok(dist_path);
    }

    let config_dir = config_path
        .parent()
        .ok_or_else(|| SpecGenError::OutputPathHasNoParent {
            config_path: config_path.to_path_buf(),
            dist: dist.to_string(),
        })?;
    Ok(config_dir.join(dist_path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_config_and_builds_minimal_protocol() {
        let config: GeneratorConfig = toml::from_str(
            r#"
            [chain]
            id = "bitshares"
            public_key_prefix = "BTS"

            [output]
            dist = "./dist/bitshares.open-graphene.json"

            [source]
            chain_repo = "../../blockchains/bitshares/bitshares-core"

            [[rpc_apis]]
            name = "database"
            class = "database_api"
            discover_method = "database"
            methods = ["get_objects"]
            "#,
        )
        .expect("parse config");

        let protocol = build_protocol(&config);
        assert_eq!(protocol.schema_version, 1);
        assert_eq!(protocol.chain.id, "bitshares");
        assert_eq!(protocol.chain.public_key_prefix, "BTS");
        assert_eq!(
            config.source.chain_repo,
            "../../blockchains/bitshares/bitshares-core"
        );
        assert_eq!(protocol.rpc_apis.len(), 1);
        assert_eq!(protocol.rpc_apis[0].name, "database");
        assert_eq!(protocol.rpc_apis[0].api_class, "database_api");
        assert_eq!(
            protocol.rpc_apis[0].discover_method.as_deref(),
            Some("database")
        );
        assert!(protocol.rpc_methods.is_empty());
    }
}
