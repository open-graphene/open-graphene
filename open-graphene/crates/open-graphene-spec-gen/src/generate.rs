use std::path::{Path, PathBuf};

use open_graphene_json_schema::{ChainDef, Protocol, RpcApiDef};

use crate::config::{GeneratorConfig, load_config};
use crate::emit::write_protocol_json;
use crate::error::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerateResult {
    pub output_path: PathBuf,
    pub rpc_api_count: usize,
    pub selected_method_count: usize,
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

pub fn generate_from_config(path: impl AsRef<Path>) -> Result<GenerateResult> {
    let config_path = path.as_ref();
    let config = load_config(config_path)?;
    let protocol = build_protocol(&config);
    let output_path = write_protocol_json(config_path, &config.output.dist, &protocol)?;

    Ok(GenerateResult {
        output_path,
        rpc_api_count: config.rpc_apis.len(),
        selected_method_count: config.rpc_apis.iter().map(|api| api.methods.len()).sum(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::GeneratorConfig;

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
