use open_graphene_json_schema::defs::RpcMethodDef;
use open_graphene_json_schema::support::{SourceMeta, SupportDef, SupportStatus};
use open_graphene_json_schema::types::TypeRef;

use crate::config::{GeneratorConfig, RpcApiConfig};
use crate::extract::{FcApi, SourceFacts, SourceLoc};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RpcResolution {
    pub methods: Vec<RpcMethodDef>,
    pub diagnostics: Vec<ResolveDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolveDiagnostic {
    pub code: String,
    pub message: String,
    pub source: Option<SourceLoc>,
}

pub fn resolve_rpc_methods(config: &GeneratorConfig, facts: &SourceFacts) -> RpcResolution {
    let mut resolution = RpcResolution::default();

    for api_config in &config.rpc_apis {
        let matching_fc_apis = matching_fc_apis(api_config, &facts.fc_apis);
        if matching_fc_apis.is_empty() {
            resolution.diagnostics.push(ResolveDiagnostic {
                code: "api_not_found_in_fc_api".to_string(),
                message: format!(
                    "configured RPC API class `{}` was not found in FC_API declarations",
                    api_config.api_class
                ),
                source: None,
            });
            continue;
        }

        for method_name in &api_config.methods {
            let Some(fc_api) = matching_fc_apis
                .iter()
                .copied()
                .find(|fc_api| fc_api.method_names.iter().any(|name| name == method_name))
            else {
                resolution.diagnostics.push(ResolveDiagnostic {
                    code: "method_not_found_in_fc_api".to_string(),
                    message: format!(
                        "configured RPC method `{}.{}` was not found in FC_API declaration for `{}`",
                        api_config.name, method_name, api_config.api_class
                    ),
                    source: matching_fc_apis.first().map(|fc_api| fc_api.source.clone()),
                });
                continue;
            };

            resolution.methods.push(provisional_rpc_method(
                api_config,
                method_name,
                &fc_api.source,
            ));
        }
    }

    resolution
}

fn matching_fc_apis<'a>(api_config: &RpcApiConfig, fc_apis: &'a [FcApi]) -> Vec<&'a FcApi> {
    fc_apis
        .iter()
        .filter(|fc_api| class_matches(&api_config.api_class, &fc_api.api_class))
        .collect()
}

fn class_matches(configured: &str, discovered: &str) -> bool {
    configured == discovered || last_path_segment(configured) == last_path_segment(discovered)
}

fn last_path_segment(value: &str) -> &str {
    value.rsplit("::").next().unwrap_or(value)
}

fn provisional_rpc_method(
    api_config: &RpcApiConfig,
    method_name: &str,
    source: &SourceLoc,
) -> RpcMethodDef {
    RpcMethodDef {
        name: method_name.to_string(),
        api_class: api_config.api_class.clone(),
        api_name: Some(api_config.name.clone()),
        params: vec![],
        returns: Some(TypeRef::AnyJson {
            reason: Some("signature extraction pending".to_string()),
            source: None,
        }),
        is_subscription: false,
        notices: vec![],
        binding_hints: None,
        source: Some(SourceMeta {
            name: Some(method_name.to_string()),
            legacy_name: None,
            file: Some(source.file.display().to_string()),
            line: Some(source.line as u32),
        }),
        support: Some(SupportDef {
            status: SupportStatus::Provisional,
            reason: Some(
                "selected in config and present in FC_API; signature extraction pending"
                    .to_string(),
            ),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ChainConfig, OutputConfig, SourceConfig};
    use std::path::PathBuf;

    #[test]
    fn resolves_configured_method_present_in_fc_api() {
        let config = config_with_methods(vec!["get_objects"]);
        let facts = facts_with_database_methods(vec!["get_objects"]);

        let resolution = resolve_rpc_methods(&config, &facts);

        assert!(resolution.diagnostics.is_empty());
        assert_eq!(resolution.methods.len(), 1);
        assert_eq!(resolution.methods[0].name, "get_objects");
        assert_eq!(resolution.methods[0].api_class, "database_api");
        assert_eq!(resolution.methods[0].api_name.as_deref(), Some("database"));
        assert!(matches!(
            resolution.methods[0].returns,
            Some(TypeRef::AnyJson { .. })
        ));
    }

    #[test]
    fn records_diagnostic_for_configured_method_missing_from_fc_api() {
        let config = config_with_methods(vec!["missing_method"]);
        let facts = facts_with_database_methods(vec!["get_objects"]);

        let resolution = resolve_rpc_methods(&config, &facts);

        assert!(resolution.methods.is_empty());
        assert_eq!(resolution.diagnostics.len(), 1);
        assert_eq!(resolution.diagnostics[0].code, "method_not_found_in_fc_api");
    }

    fn config_with_methods(methods: Vec<&str>) -> GeneratorConfig {
        GeneratorConfig {
            chain: ChainConfig {
                id: "bitshares".to_string(),
                public_key_prefix: "BTS".to_string(),
            },
            source: SourceConfig {
                chain_repo: "../../blockchains/bitshares/bitshares-core".to_string(),
            },
            output: OutputConfig {
                dist: "./dist/bitshares.open-graphene.json".to_string(),
            },
            rpc_apis: vec![RpcApiConfig {
                name: "database".to_string(),
                api_class: "database_api".to_string(),
                discover_method: Some("database".to_string()),
                methods: methods.into_iter().map(str::to_string).collect(),
            }],
        }
    }

    fn facts_with_database_methods(methods: Vec<&str>) -> SourceFacts {
        SourceFacts {
            fc_apis: vec![FcApi {
                api_class: "graphene::app::database_api".to_string(),
                method_names: methods.into_iter().map(str::to_string).collect(),
                source: SourceLoc {
                    file: PathBuf::from("database_api.hpp"),
                    line: 10,
                },
            }],
            diagnostics: vec![],
        }
    }
}
