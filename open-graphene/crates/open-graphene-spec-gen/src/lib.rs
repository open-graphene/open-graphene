//! Open Graphene protocol specification generator.
//!
//! This crate reads project-local `open-graphene.toml` files and emits
//! `open_graphene_json_schema::Protocol` JSON documents. Source-code extraction
//! will be layered on top of this config/output pipeline.

pub mod config;
pub mod emit;
pub mod error;
pub mod generate;

pub use config::{
    ChainConfig, GeneratorConfig, OutputConfig, RpcApiConfig, SourceConfig, load_config,
};
pub use error::{Result, SpecGenError};
pub use generate::{GenerateResult, build_protocol, generate_from_config};
