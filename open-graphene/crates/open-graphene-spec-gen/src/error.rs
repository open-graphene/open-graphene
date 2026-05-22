use std::path::PathBuf;

use thiserror::Error;

pub type Result<T> = std::result::Result<T, SpecGenError>;

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
    #[error("failed to read source file {path}: {source}")]
    ReadSourceFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("output path in config {config_path} must have a parent directory: {dist}")]
    OutputPathHasNoParent { config_path: PathBuf, dist: String },
    #[error("source path in config {config_path} must have a parent directory: {chain_repo}")]
    SourcePathHasNoConfigParent {
        config_path: PathBuf,
        chain_repo: String,
    },
    #[error("source chain repository does not exist or is not a directory: {path}")]
    SourceRootMissing { path: PathBuf },
    #[error("failed to discover source files under {path}: {source}")]
    DiscoverSources {
        path: PathBuf,
        #[source]
        source: walkdir::Error,
    },
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
