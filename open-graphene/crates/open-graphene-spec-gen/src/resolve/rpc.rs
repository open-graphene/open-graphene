use open_graphene_json_schema::defs::{RpcMethodDef, RpcParamDef};
use open_graphene_json_schema::support::{SourceMeta, SupportDef, SupportStatus};
use open_graphene_json_schema::types::TypeRef;

use crate::config::{GeneratorConfig, RpcApiConfig};
use crate::extract::{FcApi, RawClass, RawMethod, RawParam, SourceFacts, SourceLoc};
use crate::resolve::types::resolve_cpp_type;

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

            let raw_method = find_raw_method(api_config, method_name, facts);
            if raw_method.is_none() {
                resolution.diagnostics.push(ResolveDiagnostic {
                    code: "method_signature_not_found".to_string(),
                    message: format!(
                        "configured RPC method `{}.{}` is present in FC_API but no class method declaration was found",
                        api_config.name, method_name
                    ),
                    source: Some(fc_api.source.clone()),
                });
            }

            resolution.methods.push(provisional_rpc_method(
                api_config,
                method_name,
                &fc_api.source,
                raw_method,
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

fn find_raw_method<'a>(
    api_config: &RpcApiConfig,
    method_name: &str,
    facts: &'a SourceFacts,
) -> Option<&'a RawMethod> {
    facts
        .classes
        .iter()
        .filter(|raw_class| raw_class_matches(api_config, raw_class))
        .flat_map(|raw_class| raw_class.methods.iter())
        .find(|method| method.name == method_name)
}

fn raw_class_matches(api_config: &RpcApiConfig, raw_class: &RawClass) -> bool {
    class_matches(&api_config.api_class, &raw_class.name)
        || raw_class
            .qualified_name
            .as_deref()
            .is_some_and(|qualified| class_matches(&api_config.api_class, qualified))
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
    fc_api_source: &SourceLoc,
    raw_method: Option<&RawMethod>,
) -> RpcMethodDef {
    let source = raw_method.map_or(fc_api_source, |method| &method.source);
    let params = raw_method
        .map(rpc_params_from_raw_method)
        .unwrap_or_default();
    let returns = raw_method.map_or_else(
        || TypeRef::AnyJson {
            reason: Some("signature extraction pending".to_string()),
            source: None,
        },
        |method| resolve_rpc_return_type(api_config, method),
    );
    let support_reason = if raw_method.is_some() {
        "selected in config, present in FC_API, and signature found"
    } else {
        "selected in config and present in FC_API; signature extraction pending"
    };

    RpcMethodDef {
        name: method_name.to_string(),
        api_class: api_config.api_class.clone(),
        api_name: Some(api_config.name.clone()),
        params,
        returns: Some(returns),
        is_subscription: false,
        notices: vec![],
        binding_hints: None,
        source: Some(source_meta(method_name, source)),
        support: Some(SupportDef {
            status: SupportStatus::Provisional,
            reason: Some(support_reason.to_string()),
        }),
    }
}

fn resolve_rpc_return_type(api_config: &RpcApiConfig, method: &RawMethod) -> TypeRef {
    if api_config.name == "database" && class_matches(&api_config.api_class, "database_api") {
        if method.name == "get_objects" {
            return TypeRef::Vector {
                inner: Box::new(TypeRef::Optional {
                    inner: Box::new(TypeRef::ProtocolObjectUnion {
                        object_types: vec![],
                    }),
                }),
            };
        }
        if method.name == "get_required_fees" {
            return TypeRef::Vector {
                inner: Box::new(TypeRef::Ref {
                    name: "required_fee".to_string(),
                }),
            };
        }
        if method.name == "get_config" {
            return TypeRef::Ref {
                name: "config".to_string(),
            };
        }
    }

    resolve_cpp_type(&method.return_type)
}

fn rpc_params_from_raw_method(method: &RawMethod) -> Vec<RpcParamDef> {
    method
        .params
        .iter()
        .enumerate()
        .map(|(index, param)| rpc_param_from_raw_param(index, param, &method.source))
        .collect()
}

fn rpc_param_from_raw_param(index: usize, param: &RawParam, source: &SourceLoc) -> RpcParamDef {
    let ty = resolve_cpp_type(&param.type_expr);
    let nullable = matches!(ty, TypeRef::Optional { .. });
    RpcParamDef {
        index: index as u32,
        name: param.name.clone(),
        ty,
        required: param.default_value.is_none(),
        default_value: param
            .default_value
            .as_deref()
            .and_then(json_wire_default_value),
        nullable,
        source: Some(source_meta(&param.name, source)),
        support: Some(SupportDef {
            status: SupportStatus::Provisional,
            reason: Some("signature found".to_string()),
        }),
    }
}

fn json_wire_default_value(cpp_default: &str) -> Option<String> {
    let value = cpp_default.trim();
    if value == "true" || value == "false" || value == "null" {
        return Some(value.to_string());
    }
    if value.starts_with('"') && value.ends_with('"') {
        return Some(value.to_string());
    }
    if value.parse::<i64>().is_ok() || value.parse::<f64>().is_ok() {
        return Some(value.to_string());
    }
    None
}

fn source_meta(name: &str, source: &SourceLoc) -> SourceMeta {
    SourceMeta {
        name: Some(name.to_string()),
        legacy_name: None,
        file: Some(source.file.display().to_string()),
        line: Some(source.line as u32),
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
        let mut facts = facts_with_database_methods(vec!["get_objects"]);
        facts.classes = vec![raw_database_class_with_method("get_objects", vec![])];

        let resolution = resolve_rpc_methods(&config, &facts);

        assert!(resolution.diagnostics.is_empty());
        assert_eq!(resolution.methods.len(), 1);
        assert_eq!(resolution.methods[0].name, "get_objects");
        assert_eq!(resolution.methods[0].api_class, "database_api");
        assert_eq!(resolution.methods[0].api_name.as_deref(), Some("database"));
        assert!(matches!(
            resolution.methods[0].returns,
            Some(TypeRef::Vector { .. })
        ));
    }

    #[test]
    fn database_get_objects_returns_optional_protocol_object_union_vector() {
        let config = config_with_methods(vec!["get_objects"]);
        let mut facts = facts_with_database_methods(vec!["get_objects"]);
        facts.classes = vec![raw_database_class_with_method("get_objects", vec![])];

        let resolution = resolve_rpc_methods(&config, &facts);

        assert!(matches!(
            resolution.methods[0].returns,
            Some(TypeRef::Vector { ref inner })
                if matches!(inner.as_ref(), TypeRef::Optional { inner }
                    if matches!(inner.as_ref(), TypeRef::ProtocolObjectUnion { object_types }
                        if object_types.is_empty()))
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

    #[test]
    fn uses_raw_method_signature_for_param_names_and_defaults() {
        let config = config_with_methods(vec!["get_account_history"]);
        let mut facts = facts_with_database_methods(vec!["get_account_history"]);
        facts.classes = vec![raw_database_class_with_method(
            "get_account_history",
            vec![
                RawParam {
                    name: "account_name_or_id".to_string(),
                    type_expr: "const std::string&".to_string(),
                    default_value: None,
                },
                RawParam {
                    name: "stop".to_string(),
                    type_expr: "operation_history_id_type".to_string(),
                    default_value: Some("operation_history_id_type()".to_string()),
                },
            ],
        )];

        let resolution = resolve_rpc_methods(&config, &facts);

        assert!(resolution.diagnostics.is_empty());
        assert_eq!(resolution.methods.len(), 1);
        let method = &resolution.methods[0];
        assert_eq!(method.params.len(), 2);
        assert_eq!(method.params[0].name, "account_name_or_id");
        assert!(method.params[0].required);
        assert_eq!(method.params[1].name, "stop");
        assert!(!method.params[1].required);
        assert_eq!(method.params[1].default_value, None);
        assert_eq!(method.params[0].ty, TypeRef::String);
        assert!(matches!(method.returns, Some(TypeRef::Vector { .. })));
    }

    #[test]
    fn records_diagnostic_but_emits_method_when_signature_is_missing() {
        let config = config_with_methods(vec!["get_objects"]);
        let facts = facts_with_database_methods(vec!["get_objects"]);

        let resolution = resolve_rpc_methods(&config, &facts);

        assert_eq!(resolution.methods.len(), 1);
        assert_eq!(resolution.methods[0].params.len(), 0);
        assert_eq!(resolution.diagnostics.len(), 1);
        assert_eq!(resolution.diagnostics[0].code, "method_signature_not_found");
    }

    #[test]
    fn emits_only_safe_json_wire_default_values() {
        assert_eq!(json_wire_default_value("100"), Some("100".to_string()));
        assert_eq!(json_wire_default_value("true"), Some("true".to_string()));
        assert_eq!(
            json_wire_default_value("\"memo\""),
            Some("\"memo\"".to_string())
        );
        assert_eq!(json_wire_default_value("optional<bool>()"), None);
        assert_eq!(
            json_wire_default_value("application_options::get_default().api_limit"),
            None
        );
    }

    #[test]
    fn marks_optional_params_nullable_without_cpp_default_value() {
        let param = RawParam {
            name: "subscribe".to_string(),
            type_expr: "optional<bool>".to_string(),
            default_value: Some("optional<bool>()".to_string()),
        };

        let rpc_param = rpc_param_from_raw_param(
            0,
            &param,
            &SourceLoc {
                file: PathBuf::from("database_api.hpp"),
                line: 20,
            },
        );

        assert!(!rpc_param.required);
        assert!(rpc_param.nullable);
        assert_eq!(rpc_param.default_value, None);
        assert_eq!(
            rpc_param.ty,
            TypeRef::Optional {
                inner: Box::new(TypeRef::Bool)
            }
        );
    }

    fn config_with_methods(methods: Vec<&str>) -> GeneratorConfig {
        GeneratorConfig {
            chain: ChainConfig {
                id: "bitshares".to_string(),
                public_key_prefix: "BTS".to_string(),
                chain_id: None,
            },
            source: SourceConfig {
                chain_repo: "../../blockchains/bitshares/bitshares-core".to_string(),
            },
            output: OutputConfig {
                dist: "./dist/bitshares.open-graphene.json".to_string(),
            },
            object_structs: vec![],
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
            classes: vec![],
            enums: vec![],
            static_variants: vec![],
            object_types: vec![],
            reflects: vec![],
            virtual_operations: vec![],
            diagnostics: vec![],
        }
    }

    fn raw_database_class_with_method(method_name: &str, params: Vec<RawParam>) -> RawClass {
        RawClass {
            name: "database_api".to_string(),
            qualified_name: Some("graphene::app::database_api".to_string()),
            methods: vec![RawMethod {
                name: method_name.to_string(),
                return_type: "vector<operation_history_object>".to_string(),
                params,
                is_const: true,
                source: SourceLoc {
                    file: PathBuf::from("database_api.hpp"),
                    line: 20,
                },
            }],
            fields: vec![],
            source: SourceLoc {
                file: PathBuf::from("database_api.hpp"),
                line: 5,
            },
        }
    }
}
