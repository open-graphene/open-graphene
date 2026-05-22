use std::path::PathBuf;

use thiserror::Error;

pub type Result<T> = std::result::Result<T, GenBindingsRsError>;

#[derive(Debug, Error)]
pub enum GenBindingsRsError {
    #[error("failed to read protocol spec {path}: {source}")]
    ReadSpec {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse protocol spec JSON {path}: {source}")]
    ParseSpec {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("failed to create output directory {path}: {source}")]
    CreateOutDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to write generated Rust bindings {path}: {source}")]
    WriteOutput {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to render generated Rust bindings: {message}")]
    Render { message: String },
}
