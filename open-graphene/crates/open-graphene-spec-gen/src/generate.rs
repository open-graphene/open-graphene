use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use open_graphene_json_schema::defs::{FieldDef, RpcApiDef, RpcMethodDef, StructDef, StructKind};
use open_graphene_json_schema::support::{SourceMeta, SupportDef, SupportStatus};
use open_graphene_json_schema::types::TypeRef;
use open_graphene_json_schema::{ChainDef, Protocol};

use crate::config::{GeneratorConfig, load_config};
use crate::emit::write_protocol_json;
use crate::error::Result;
use crate::extract::{RawClass, SourceFacts, SourceLoc, extract_source_facts};
use crate::resolve::{resolve_cpp_type, resolve_rpc_methods};
use crate::source::discover_sources;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerateResult {
    pub output_path: PathBuf,
    pub rpc_api_count: usize,
    pub selected_method_count: usize,
    pub source_file_count: usize,
    pub fc_api_count: usize,
    pub class_count: usize,
    pub method_declaration_count: usize,
    pub field_declaration_count: usize,
    pub resolved_struct_count: usize,
    pub resolved_rpc_method_count: usize,
    pub diagnostic_count: usize,
}

pub fn build_protocol(
    config: &GeneratorConfig,
    facts: &SourceFacts,
    rpc_methods: Vec<RpcMethodDef>,
) -> Protocol {
    let structs = build_referenced_structs(facts, &rpc_methods);

    Protocol {
        schema_version: 1,
        chain: ChainDef {
            id: config.chain.id.clone(),
            public_key_prefix: config.chain.public_key_prefix.clone(),
        },
        structs,
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
        rpc_methods,
        strict_mode: None,
    }
}

fn build_referenced_structs(facts: &SourceFacts, rpc_methods: &[RpcMethodDef]) -> Vec<StructDef> {
    let mut refs = BTreeSet::new();
    for method in rpc_methods {
        if let Some(returns) = &method.returns {
            collect_type_refs(returns, &mut refs);
        }
        for param in &method.params {
            collect_type_refs(&param.ty, &mut refs);
        }
    }

    refs.into_iter()
        .filter_map(|name| find_raw_class(facts, &name).and_then(raw_class_to_struct_def))
        .collect()
}

fn collect_type_refs(ty: &TypeRef, refs: &mut BTreeSet<String>) {
    match ty {
        TypeRef::Ref { name } => {
            refs.insert(name.clone());
        }
        TypeRef::Optional { inner } | TypeRef::Vector { inner } | TypeRef::Set { inner, .. } => {
            collect_type_refs(inner, refs)
        }
        TypeRef::Map { key, value, .. } | TypeRef::FlatMap { key, value, .. } => {
            collect_type_refs(key, refs);
            collect_type_refs(value, refs);
        }
        TypeRef::Pair { first, second } => {
            collect_type_refs(first, refs);
            collect_type_refs(second, refs);
        }
        _ => {}
    }
}

fn find_raw_class<'a>(facts: &'a SourceFacts, name: &str) -> Option<&'a RawClass> {
    facts.classes.iter().find(|class| class.name == name)
}

fn raw_class_to_struct_def(class: &RawClass) -> Option<StructDef> {
    if class.fields.is_empty() {
        return None;
    }

    Some(StructDef {
        name: class.name.clone(),
        source_name: class.qualified_name.clone(),
        kind: StructKind::Struct,
        wire_tag: None,
        fields: class
            .fields
            .iter()
            .enumerate()
            .map(|(index, field)| FieldDef {
                index: index as u32,
                name: field.name.clone(),
                ty: resolve_cpp_type(&field.type_expr),
                source: Some(source_meta(&field.name, &field.source)),
                support: Some(SupportDef {
                    status: SupportStatus::Provisional,
                    reason: Some(
                        "class field declaration extracted; reflection coverage pending"
                            .to_string(),
                    ),
                }),
            })
            .collect(),
        support: Some(SupportDef {
            status: SupportStatus::Provisional,
            reason: Some(
                "class fields extracted from C++ declaration; FC_REFLECT validation pending"
                    .to_string(),
            ),
        }),
    })
}

fn source_meta(name: &str, source: &SourceLoc) -> SourceMeta {
    SourceMeta {
        name: Some(name.to_string()),
        legacy_name: None,
        file: Some(source.file.display().to_string()),
        line: Some(source.line as u32),
    }
}

pub fn generate_from_config(path: impl AsRef<Path>) -> Result<GenerateResult> {
    let config_path = path.as_ref();
    let config = load_config(config_path)?;
    let source_set = discover_sources(config_path, &config.source)?;
    let facts = extract_source_facts(&source_set)?;
    let rpc_resolution = resolve_rpc_methods(&config, &facts);
    let resolved_rpc_method_count = rpc_resolution.methods.len();
    let class_count = facts.classes.len();
    let method_declaration_count = facts.classes.iter().map(|class| class.methods.len()).sum();
    let field_declaration_count = facts.classes.iter().map(|class| class.fields.len()).sum();
    let diagnostic_count = facts.diagnostics.len() + rpc_resolution.diagnostics.len();
    let protocol = build_protocol(&config, &facts, rpc_resolution.methods);
    let resolved_struct_count = protocol.structs.len();
    let output_path = write_protocol_json(config_path, &config.output.dist, &protocol)?;

    Ok(GenerateResult {
        output_path,
        rpc_api_count: config.rpc_apis.len(),
        selected_method_count: config.rpc_apis.iter().map(|api| api.methods.len()).sum(),
        source_file_count: source_set.files.len(),
        fc_api_count: facts.fc_apis.len(),
        class_count,
        method_declaration_count,
        field_declaration_count,
        resolved_struct_count,
        resolved_rpc_method_count,
        diagnostic_count,
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

        let facts = SourceFacts::default();
        let protocol = build_protocol(&config, &facts, vec![]);
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
