//! Open Graphene protocol specification generator.
//!
//! This crate reads project-local `open-graphene.toml` files and emits
//! `open_graphene_json_schema::Protocol` JSON documents. Source-code extraction
//! will be layered on top of this config/output pipeline.

pub mod config;
pub mod emit;
pub mod error;
pub mod extract;
pub mod generate;
pub mod resolve;
pub mod source;

pub use config::{
    ChainConfig, GeneratorConfig, OutputConfig, RpcApiConfig, SourceConfig, load_config,
};
pub use error::{Result, SpecGenError};
pub use extract::{
    ExtractDiagnostic, FcApi, RawClass, RawMethod, RawParam, SourceFacts, SourceLoc,
    extract_source_facts,
};
pub use generate::{GenerateResult, build_protocol, generate_from_config};
pub use resolve::{ResolveDiagnostic, RpcResolution, resolve_rpc_methods};
pub use source::{SourceFile, SourceFileKind, SourceSet, discover_sources, resolve_chain_repo};
