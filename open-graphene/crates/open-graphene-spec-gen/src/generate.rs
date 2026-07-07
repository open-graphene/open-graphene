use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use open_graphene_json_schema::defs::{
    EnumDef, EnumValueDef, FieldDef, ObjectTypeDef, OperationDef, RpcApiDef, RpcMethodDef,
    StaticVariantArmDef, StaticVariantDef, StructDef, StructKind,
};
use open_graphene_json_schema::support::{SourceMeta, SupportDef, SupportStatus};
use open_graphene_json_schema::types::{IntType, TypeRef};
use open_graphene_json_schema::{ChainDef, Protocol};

use crate::config::{GeneratorConfig, load_config};
use crate::emit::write_protocol_json;
use crate::error::Result;
use crate::extract::{
    RawClass, RawEnum, RawObjectType, RawReflect, RawStaticVariant, SourceFacts, SourceLoc,
    extract_source_facts,
};
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
    pub static_variant_count: usize,
    pub resolved_struct_count: usize,
    pub resolved_static_variant_count: usize,
    pub resolved_rpc_method_count: usize,
    pub diagnostic_count: usize,
}

pub fn build_protocol(
    config: &GeneratorConfig,
    facts: &SourceFacts,
    mut rpc_methods: Vec<RpcMethodDef>,
) -> Protocol {
    let (structs, static_variants) = build_type_graph(config, facts, &rpc_methods);
    let operations = build_operations(&static_variants, &structs, &facts.virtual_operations);
    let object_types = build_object_types(facts, &structs);
    let enums = build_enums(facts);
    populate_protocol_object_unions(&mut rpc_methods, &object_types);

    Protocol {
        schema_version: 1,
        chain: ChainDef {
            id: config.chain.id.clone(),
            public_key_prefix: config.chain.public_key_prefix.clone(),
            chain_id: config.chain.chain_id.clone(),
        },
        structs,
        enums,
        static_variants,
        operations,
        object_types,
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

fn build_type_graph(
    config: &GeneratorConfig,
    facts: &SourceFacts,
    rpc_methods: &[RpcMethodDef],
) -> (Vec<StructDef>, Vec<StaticVariantDef>) {
    let mut struct_refs = BTreeSet::new();
    let mut static_variant_refs = BTreeSet::new();
    let mut resolved_structs = BTreeSet::new();
    let mut resolved_static_variants = BTreeSet::new();
    let mut structs = Vec::new();
    let mut static_variants = Vec::new();

    for method in rpc_methods {
        if let Some(returns) = &method.returns {
            collect_refs_from_type(returns, &mut struct_refs, &mut static_variant_refs);
        }
        for param in &method.params {
            collect_refs_from_type(&param.ty, &mut struct_refs, &mut static_variant_refs);
        }
    }
    seed_protocol_root_struct_refs(&mut struct_refs);
    seed_configured_object_struct_refs(config, facts, &mut struct_refs);

    loop {
        let mut changed = false;

        let pending_static_variants = static_variant_refs
            .iter()
            .filter(|name| !resolved_static_variants.contains(*name))
            .cloned()
            .collect::<Vec<_>>();
        for name in pending_static_variants {
            resolved_static_variants.insert(name.clone());
            let def = if let Some(raw) = find_raw_static_variant(facts, &name) {
                raw_static_variant_to_def(raw)
            } else if name == "fee_parameters" {
                let Some(def) = fee_parameters_static_variant_to_def(facts) else {
                    continue;
                };
                def
            } else {
                continue;
            };
            for arm in &def.variants {
                collect_refs_from_type(&arm.ty, &mut struct_refs, &mut static_variant_refs);
            }
            static_variants.push(def);
            changed = true;
        }

        let pending_structs = struct_refs
            .iter()
            .filter(|name| !resolved_structs.contains(*name))
            .cloned()
            .collect::<Vec<_>>();
        for name in pending_structs {
            resolved_structs.insert(name.clone());
            let def = if let Some(raw) = find_raw_class(facts, &name) {
                raw_class_to_struct_def(raw, facts)
            } else if let Some(def) = known_core_struct_def(&name) {
                def
            } else if let Some(def) = nested_reflect_struct_to_def(facts, &name) {
                def
            } else {
                continue;
            };
            for field in &def.fields {
                collect_refs_from_type(&field.ty, &mut struct_refs, &mut static_variant_refs);
            }
            structs.push(def);
            changed = true;
        }

        if !changed {
            break;
        }
    }

    structs.sort_by(|left, right| left.name.cmp(&right.name));
    static_variants.sort_by(|left, right| left.name.cmp(&right.name));
    (structs, static_variants)
}

fn seed_protocol_root_struct_refs(struct_refs: &mut BTreeSet<String>) {
    struct_refs.insert("transaction".to_string());
    struct_refs.insert("signed_transaction".to_string());
}

fn seed_configured_object_struct_refs(
    config: &GeneratorConfig,
    facts: &SourceFacts,
    struct_refs: &mut BTreeSet<String>,
) {
    for object_struct in &config.object_structs {
        if facts
            .object_types
            .iter()
            .any(|object_type| object_type.struct_ref.as_ref() == Some(object_struct))
        {
            struct_refs.insert(object_struct.clone());
        }
    }
}

fn build_enums(facts: &SourceFacts) -> Vec<EnumDef> {
    let mut enums = facts.enums.iter().map(raw_enum_to_def).collect::<Vec<_>>();
    enums.sort_by(|left, right| left.name.cmp(&right.name));
    enums
}

fn raw_enum_to_def(raw: &RawEnum) -> EnumDef {
    let name = last_path_segment(&raw.name).to_string();
    EnumDef {
        is_bitfield: is_bitfield_enum(&name),
        name,
        underlying: IntType::I32,
        values: raw
            .values
            .iter()
            .map(|value| EnumValueDef {
                name: value.name.clone(),
                value: value.value,
            })
            .collect(),
        source: Some(source_meta(&raw.name, &raw.source)),
        support: Some(SupportDef {
            status: SupportStatus::Provisional,
            reason: Some("enum extracted from FC_REFLECT_ENUM".to_string()),
        }),
    }
}

fn is_bitfield_enum(name: &str) -> bool {
    name.contains("flags") || name.contains("permission")
}

fn build_operations(
    static_variants: &[StaticVariantDef],
    structs: &[StructDef],
    virtual_operations: &[String],
) -> Vec<OperationDef> {
    let Some(operation_variant) = static_variants
        .iter()
        .find(|variant| variant.name == "operation")
    else {
        return vec![];
    };

    let mut operations = operation_variant
        .variants
        .iter()
        .filter_map(|arm| operation_from_arm(arm, structs, virtual_operations))
        .collect::<Vec<_>>();
    operations.sort_by(|left, right| left.wire_tag.cmp(&right.wire_tag));
    operations
}

fn operation_from_arm(
    arm: &StaticVariantArmDef,
    structs: &[StructDef],
    virtual_operations: &[String],
) -> Option<OperationDef> {
    let TypeRef::Ref { name } = &arm.ty else {
        return None;
    };
    let struct_def = structs.iter().find(|struct_def| &struct_def.name == name)?;

    Some(OperationDef {
        name: arm.name.clone(),
        wire_tag: arm.tag,
        fields: struct_def.fields.clone(),
        is_virtual: virtual_operations.iter().any(|marked| marked == &arm.name),
        source: None,
        support: Some(SupportDef {
            status: SupportStatus::Provisional,
            reason: Some(
                "operation derived from operation static_variant arm and matching struct fields"
                    .to_string(),
            ),
        }),
    })
}

fn build_object_types(facts: &SourceFacts, structs: &[StructDef]) -> Vec<ObjectTypeDef> {
    let struct_names = structs
        .iter()
        .map(|struct_def| struct_def.name.as_str())
        .collect::<BTreeSet<_>>();
    let mut object_types = facts
        .object_types
        .iter()
        .map(|raw| raw_object_type_to_def(raw, &struct_names))
        .collect::<Vec<_>>();

    object_types.sort_by(|left, right| {
        left.object_space
            .cmp(&right.object_space)
            .then(left.type_id.cmp(&right.type_id))
            .then(left.object_type.cmp(&right.object_type))
    });
    object_types
}

fn raw_object_type_to_def(raw: &RawObjectType, struct_names: &BTreeSet<&str>) -> ObjectTypeDef {
    let struct_ref = raw
        .struct_ref
        .as_deref()
        .filter(|name| struct_names.contains(name))
        .map(ToString::to_string);

    ObjectTypeDef {
        object_type: raw.object_type.clone(),
        cpp_alias: raw.cpp_alias.clone(),
        object_space: raw.object_space,
        type_id: raw.type_id,
        struct_ref,
        source: Some(source_meta(&raw.object_type, &raw.source)),
        support: Some(SupportDef {
            status: SupportStatus::Provisional,
            reason: Some(format!(
                "object type extracted from GRAPHENE_DEFINE_IDS in namespace `{}` as `{}` from space `{}`",
                raw.id_namespace, raw.object_type_name, raw.object_space_name
            )),
        }),
    }
}

fn populate_protocol_object_unions(
    rpc_methods: &mut [RpcMethodDef],
    object_types: &[ObjectTypeDef],
) {
    let known_object_types = object_types
        .iter()
        .map(|object_type| object_type.object_type.clone())
        .collect::<Vec<_>>();

    for method in rpc_methods {
        if let Some(returns) = &mut method.returns {
            populate_protocol_object_union(returns, &known_object_types);
        }
        for param in &mut method.params {
            populate_protocol_object_union(&mut param.ty, &known_object_types);
        }
    }
}

fn populate_protocol_object_union(ty: &mut TypeRef, known_object_types: &[String]) {
    match ty {
        TypeRef::ProtocolObjectUnion { object_types } if object_types.is_empty() => {
            *object_types = known_object_types.to_vec();
        }
        TypeRef::Optional { inner } | TypeRef::Vector { inner } | TypeRef::Set { inner, .. } => {
            populate_protocol_object_union(inner, known_object_types)
        }
        TypeRef::Map { key, value, .. } | TypeRef::FlatMap { key, value, .. } => {
            populate_protocol_object_union(key, known_object_types);
            populate_protocol_object_union(value, known_object_types);
        }
        TypeRef::Pair { first, second } => {
            populate_protocol_object_union(first, known_object_types);
            populate_protocol_object_union(second, known_object_types);
        }
        _ => {}
    }
}

fn collect_refs_from_type(
    ty: &TypeRef,
    struct_refs: &mut BTreeSet<String>,
    static_variant_refs: &mut BTreeSet<String>,
) {
    match ty {
        TypeRef::Ref { name } => {
            struct_refs.insert(name.clone());
        }
        TypeRef::StaticVariantRef { name } => {
            static_variant_refs.insert(name.clone());
        }
        TypeRef::Optional { inner } | TypeRef::Vector { inner } | TypeRef::Set { inner, .. } => {
            collect_refs_from_type(inner, struct_refs, static_variant_refs)
        }
        TypeRef::Map { key, value, .. } | TypeRef::FlatMap { key, value, .. } => {
            collect_refs_from_type(key, struct_refs, static_variant_refs);
            collect_refs_from_type(value, struct_refs, static_variant_refs);
        }
        TypeRef::Pair { first, second } => {
            collect_refs_from_type(first, struct_refs, static_variant_refs);
            collect_refs_from_type(second, struct_refs, static_variant_refs);
        }
        _ => {}
    }
}

fn find_raw_static_variant<'a>(facts: &'a SourceFacts, name: &str) -> Option<&'a RawStaticVariant> {
    facts
        .static_variants
        .iter()
        .find(|variant| variant.name == name)
}

fn raw_static_variant_to_def(raw: &RawStaticVariant) -> StaticVariantDef {
    StaticVariantDef {
        name: raw.name.clone(),
        kind: "fc_static_variant".to_string(),
        json: "tagged_tuple".to_string(),
        fc: "static_variant".to_string(),
        variants: raw
            .variants
            .iter()
            .enumerate()
            .map(|(index, variant)| StaticVariantArmDef {
                tag: index as u32,
                name: variant.clone(),
                ty: resolve_cpp_type(variant),
                support: Some(SupportDef {
                    status: SupportStatus::Provisional,
                    reason: Some("static variant arm extracted from C++ type list".to_string()),
                }),
            })
            .collect(),
        source: Some(source_meta(&raw.name, &raw.source)),
        support: Some(SupportDef {
            status: SupportStatus::Provisional,
            reason: Some("static variant extracted from C++ type list".to_string()),
        }),
    }
}

fn fee_parameters_static_variant_to_def(facts: &SourceFacts) -> Option<StaticVariantDef> {
    let operation = find_raw_static_variant(facts, "operation")?;
    Some(StaticVariantDef {
        name: "fee_parameters".to_string(),
        kind: "fc_static_variant".to_string(),
        json: "tagged_tuple".to_string(),
        fc: "static_variant".to_string(),
        variants: operation
            .variants
            .iter()
            .enumerate()
            .map(|(index, operation_name)| {
                let fee_params_name = nested_struct_name(operation_name, "fee_params_t");
                StaticVariantArmDef {
                    tag: index as u32,
                    name: fee_params_name.clone(),
                    ty: TypeRef::Ref {
                        name: fee_params_name,
                    },
                    support: Some(SupportDef {
                        status: SupportStatus::Provisional,
                        reason: Some(
                            "fee_parameters arm synthesized from operation static_variant arm and nested fee_params_t"
                                .to_string(),
                        ),
                    }),
                }
            })
            .collect(),
        source: Some(source_meta("fee_parameters", &operation.source)),
        support: Some(SupportDef {
            status: SupportStatus::Provisional,
            reason: Some(
                "fee_parameters synthesized from transform_to_fee_parameters<operation>".to_string(),
            ),
        }),
    })
}

fn find_raw_class<'a>(facts: &'a SourceFacts, name: &str) -> Option<&'a RawClass> {
    // Unqualified type references resolve to top-level classes; a nested class
    // with the same short name (extracted from an earlier-sorted file) must
    // not shadow them. Names that only exist nested still resolve as before.
    facts
        .classes
        .iter()
        .find(|class| class.name == name && class.qualified_name.is_none())
        .or_else(|| facts.classes.iter().find(|class| class.name == name))
}

fn raw_class_to_struct_def(class: &RawClass, facts: &SourceFacts) -> StructDef {
    let reflect = find_raw_reflect(facts, class);
    let mut fields = reflected_or_declared_fields(class, reflect, facts);
    prepend_inherited_object_id_field(class, facts, &mut fields);
    let support_reason = if reflect.is_some() {
        "fields ordered and filtered by FC_REFLECT; field declarations provide types"
    } else {
        "class fields extracted from C++ declaration; FC_REFLECT validation pending"
    };

    StructDef {
        name: class.name.clone(),
        source_name: class.qualified_name.clone(),
        kind: StructKind::Struct,
        wire_tag: None,
        fields,
        support: Some(SupportDef {
            status: SupportStatus::Provisional,
            reason: Some(support_reason.to_string()),
        }),
    }
}

fn prepend_inherited_object_id_field(
    class: &RawClass,
    facts: &SourceFacts,
    fields: &mut Vec<FieldDef>,
) {
    if fields.iter().any(|field| field.name == "id") {
        return;
    }

    let Some(object_type) = facts
        .object_types
        .iter()
        .find(|object_type| object_type.struct_ref.as_deref() == Some(class.name.as_str()))
    else {
        return;
    };

    fields.insert(
        0,
        FieldDef {
            index: 0,
            name: "id".to_string(),
            ty: TypeRef::ProtocolObjectId {
                object_type: object_type.object_type.clone(),
            },
            source: Some(source_meta("id", &class.source)),
            support: Some(SupportDef {
                status: SupportStatus::Provisional,
                reason: Some(
                    "inherited object id from graphene::db::object for reflected Graphene object"
                        .to_string(),
                ),
            }),
        },
    );

    for (index, field) in fields.iter_mut().enumerate() {
        field.index = index as u32;
    }
}

fn find_raw_reflect<'a>(facts: &'a SourceFacts, class: &RawClass) -> Option<&'a RawReflect> {
    match class.qualified_name.as_deref() {
        // A nested class only matches reflects that spell out its nesting
        // path, e.g. `stealth_confirmation::memo_data` never takes the
        // reflect of a top-level `memo_data`.
        Some(qualified) => facts
            .reflects
            .iter()
            .find(|reflect| reflect_path_matches(qualified, &reflect.type_name)),
        // A top-level class must not take a nested class's reflect either:
        // the last-segment fallback would otherwise match
        // `graphene::protocol::stealth_confirmation::memo_data` to the
        // top-level `memo_data`.
        None => facts.reflects.iter().find(|reflect| {
            reflect_type_matches(class.name.as_str(), &reflect.type_name)
                && !reflect_claimed_by_nested_class(facts, &reflect.type_name)
        }),
    }
}

fn reflect_path_matches(qualified_class: &str, reflect_type_name: &str) -> bool {
    reflect_type_name == qualified_class
        || reflect_type_name
            .strip_suffix(qualified_class)
            .is_some_and(|prefix| prefix.ends_with("::"))
}

fn reflect_claimed_by_nested_class(facts: &SourceFacts, reflect_type_name: &str) -> bool {
    facts.classes.iter().any(|class| {
        class
            .qualified_name
            .as_deref()
            .is_some_and(|qualified| reflect_path_matches(qualified, reflect_type_name))
    })
}

fn reflect_type_matches(class_name: &str, reflect_type_name: &str) -> bool {
    class_name == reflect_type_name
        || last_path_segment(class_name) == last_path_segment(reflect_type_name)
}

fn last_path_segment(value: &str) -> &str {
    value.rsplit("::").next().unwrap_or(value)
}

fn reflected_or_declared_fields(
    class: &RawClass,
    reflect: Option<&RawReflect>,
    facts: &SourceFacts,
) -> Vec<FieldDef> {
    let Some(reflect) = reflect else {
        return class
            .fields
            .iter()
            .enumerate()
            .map(|(index, field)| {
                field_def(
                    index,
                    field,
                    class,
                    facts,
                    "class field declaration extracted; reflection coverage pending",
                )
            })
            .collect();
    };

    let mut fields = Vec::new();
    for base in &reflect.bases {
        if let Some(base_class) = find_raw_class(facts, last_path_segment(base)) {
            let base_reflect = find_raw_reflect(facts, base_class);
            fields.extend(reflected_or_declared_fields(
                base_class,
                base_reflect,
                facts,
            ));
        }
    }

    fields.extend(
        reflect
            .fields
            .iter()
            .filter_map(|field_name| class.fields.iter().find(|field| &field.name == field_name))
            .enumerate()
            .map(|(index, field)| {
                field_def(
                    index,
                    field,
                    class,
                    facts,
                    "field selected and ordered by FC_REFLECT",
                )
            }),
    );

    for (index, field) in fields.iter_mut().enumerate() {
        field.index = index as u32;
    }
    fields
}

fn field_def(
    index: usize,
    field: &crate::extract::RawField,
    class: &RawClass,
    facts: &SourceFacts,
    support_reason: &str,
) -> FieldDef {
    FieldDef {
        index: index as u32,
        name: field.name.clone(),
        ty: resolve_field_type(&field.type_expr, class, facts),
        source: Some(source_meta(&field.name, &field.source)),
        support: Some(SupportDef {
            status: SupportStatus::Provisional,
            reason: Some(support_reason.to_string()),
        }),
    }
}

fn resolve_field_type(type_expr: &str, class: &RawClass, facts: &SourceFacts) -> TypeRef {
    if let Some(inner) = extension_inner(type_expr)
        && nested_reflect_for(&class.name, inner, facts).is_some()
    {
        return TypeRef::Ref {
            name: nested_struct_name(&class.name, inner),
        };
    }

    resolve_cpp_type(type_expr)
}

fn extension_inner(type_expr: &str) -> Option<&str> {
    let value = type_expr.trim();
    let rest = value.strip_prefix("extension<")?;
    let inner = rest.strip_suffix('>')?.trim();
    if inner.is_empty() { None } else { Some(inner) }
}

fn known_core_struct_def(name: &str) -> Option<StructDef> {
    let (source_name, support_reason, fields): (&str, &str, Vec<(&str, TypeRef)>) = match name {
        "immutable_chain_parameters" => (
            "graphene::chain::immutable_chain_parameters",
            "manual core type mapping; header uses FC_REFLECT_TYPENAME without field reflection",
            vec![
                ("min_committee_member_count", TypeRef::Uint16),
                ("min_witness_count", TypeRef::Uint16),
                ("num_special_accounts", TypeRef::Uint32),
                ("num_special_assets", TypeRef::Uint32),
            ],
        ),
        "range_proof_info" => (
            "fc::ecc::range_proof_info",
            "manual RPC DTO mapping; fc::ecc::range_proof_info is returned by crypto.range_get_info",
            vec![
                (
                    "exp",
                    TypeRef::Int32 {
                        fc: None,
                        source: None,
                    },
                ),
                (
                    "mantissa",
                    TypeRef::Int32 {
                        fc: None,
                        source: None,
                    },
                ),
                (
                    "min_value",
                    TypeRef::Uint64 {
                        json: None,
                        fc: None,
                    },
                ),
                (
                    "max_value",
                    TypeRef::Uint64 {
                        json: None,
                        fc: None,
                    },
                ),
            ],
        ),
        _ => return None,
    };

    let fields = fields
        .into_iter()
        .enumerate()
        .map(|(index, (name, ty))| FieldDef {
            index: index as u32,
            name: name.to_string(),
            ty,
            source: None,
            support: Some(SupportDef {
                status: SupportStatus::Supported,
                reason: Some(support_reason.to_string()),
            }),
        })
        .collect();

    Some(StructDef {
        name: name.to_string(),
        source_name: Some(source_name.to_string()),
        kind: StructKind::Struct,
        wire_tag: None,
        fields,
        support: Some(SupportDef {
            status: SupportStatus::Supported,
            reason: Some(support_reason.to_string()),
        }),
    })
}

fn nested_reflect_for<'a>(
    parent: &str,
    nested: &str,
    facts: &'a SourceFacts,
) -> Option<&'a RawReflect> {
    let suffix = format!("::{parent}::{nested}");
    facts
        .reflects
        .iter()
        .find(|reflect| reflect.type_name.ends_with(&suffix))
}

fn nested_struct_name(parent: &str, nested: &str) -> String {
    format!("{parent}_{nested}")
}

fn nested_reflect_struct_to_def(facts: &SourceFacts, name: &str) -> Option<StructDef> {
    let reflect = facts.reflects.iter().find(|reflect| {
        nested_reflect_synthetic_name(&reflect.type_name)
            .as_deref()
            .is_some_and(|synthetic| synthetic == name)
    })?;
    let nested_name = last_path_segment(&reflect.type_name);
    let raw = facts
        .classes
        .iter()
        .filter(|class| class.name == nested_name && class.source.file == reflect.source.file)
        .filter(|class| class.source.line <= reflect.source.line)
        .max_by_key(|class| class.source.line)?;

    Some(StructDef {
        name: name.to_string(),
        source_name: Some(reflect.type_name.clone()),
        kind: StructKind::Struct,
        wire_tag: None,
        fields: reflected_or_declared_fields(raw, Some(reflect), facts),
        support: Some(SupportDef {
            status: SupportStatus::Provisional,
            reason: Some(
                "nested struct synthesized from qualified FC_REFLECT and matching nested declaration"
                    .to_string(),
            ),
        }),
    })
}

fn nested_reflect_synthetic_name(type_name: &str) -> Option<String> {
    let mut segments = type_name.rsplit("::");
    let nested = segments.next()?;
    let parent = segments.next()?;
    Some(nested_struct_name(parent, nested))
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
    let static_variant_count = facts.static_variants.len();
    let diagnostic_count = facts.diagnostics.len() + rpc_resolution.diagnostics.len();
    let protocol = build_protocol(&config, &facts, rpc_resolution.methods);
    let resolved_struct_count = protocol.structs.len();
    let resolved_static_variant_count = protocol.static_variants.len();
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
        static_variant_count,
        resolved_struct_count,
        resolved_static_variant_count,
        resolved_rpc_method_count,
        diagnostic_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::GeneratorConfig;
    use crate::extract::{RawEnumValue, RawField};
    use open_graphene_json_schema::OrderingRule;

    #[test]
    fn build_protocol_emits_enums_from_reflect_enum() {
        let config: GeneratorConfig = toml::from_str(
            r#"
            [chain]
            id = "bitshares"
            public_key_prefix = "BTS"

            [output]
            dist = "./dist/bitshares.open-graphene.json"

            [source]
            chain_repo = "../../blockchains/bitshares/bitshares-core"
            "#,
        )
        .expect("parse config");
        let facts = SourceFacts {
            enums: vec![RawEnum {
                name: "graphene::protocol::restriction::function_type".to_string(),
                values: vec![
                    RawEnumValue {
                        name: "func_eq".to_string(),
                        value: 0,
                    },
                    RawEnumValue {
                        name: "func_ne".to_string(),
                        value: 1,
                    },
                ],
                source: SourceLoc {
                    file: PathBuf::from("restriction.hpp"),
                    line: 117,
                },
            }],
            ..SourceFacts::default()
        };

        let protocol = build_protocol(&config, &facts, vec![]);

        assert_eq!(protocol.enums.len(), 1);
        assert_eq!(protocol.enums[0].name, "function_type");
        assert_eq!(protocol.enums[0].underlying, IntType::I32);
        assert_eq!(protocol.enums[0].values[0].name, "func_eq");
        assert_eq!(protocol.enums[0].values[1].value, 1);
    }

    #[test]
    fn build_protocol_emits_object_types_and_populates_protocol_object_union() {
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
        let facts = SourceFacts {
            object_types: vec![RawObjectType {
                object_type: "account".to_string(),
                cpp_alias: "account_id_type".to_string(),
                object_space_name: "protocol_ids".to_string(),
                object_space: Some(1),
                object_type_name: "account_object_type".to_string(),
                type_id: Some(2),
                struct_ref: Some("account_object".to_string()),
                source: SourceLoc {
                    file: PathBuf::from("types.hpp"),
                    line: 10,
                },
                id_namespace: "protocol".to_string(),
            }],
            ..SourceFacts::default()
        };
        let rpc_methods = vec![RpcMethodDef {
            name: "get_objects".to_string(),
            api_class: "database_api".to_string(),
            api_name: Some("database".to_string()),
            params: vec![],
            returns: Some(TypeRef::Vector {
                inner: Box::new(TypeRef::Optional {
                    inner: Box::new(TypeRef::ProtocolObjectUnion {
                        object_types: vec![],
                    }),
                }),
            }),
            is_subscription: false,
            notices: vec![],
            binding_hints: None,
            source: None,
            support: None,
        }];

        let protocol = build_protocol(&config, &facts, rpc_methods);

        assert_eq!(protocol.object_types.len(), 1);
        assert_eq!(protocol.object_types[0].object_type, "account");
        let Some(TypeRef::Vector { inner }) = &protocol.rpc_methods[0].returns else {
            panic!("expected vector return");
        };
        let TypeRef::Optional { inner } = inner.as_ref() else {
            panic!("expected optional inner");
        };
        let TypeRef::ProtocolObjectUnion { object_types } = inner.as_ref() else {
            panic!("expected protocol object union");
        };
        assert_eq!(object_types, &vec!["account".to_string()]);
    }

    #[test]
    fn build_protocol_adds_inherited_id_to_object_structs() {
        let config: GeneratorConfig = toml::from_str(
            r#"
            [chain]
            id = "bitshares"
            public_key_prefix = "BTS"

            [output]
            dist = "./dist/bitshares.open-graphene.json"

            [source]
            chain_repo = "../../blockchains/bitshares/bitshares-core"
            "#,
        )
        .expect("parse config");
        let facts = SourceFacts {
            classes: vec![RawClass {
                name: "limit_order_object".to_string(),
                qualified_name: Some("graphene::chain::limit_order_object".to_string()),
                methods: vec![],
                fields: vec![RawField {
                    name: "seller".to_string(),
                    type_expr: "account_id_type".to_string(),
                    source: SourceLoc {
                        file: PathBuf::from("market_object.hpp"),
                        line: 47,
                    },
                }],
                source: SourceLoc {
                    file: PathBuf::from("market_object.hpp"),
                    line: 45,
                },
            }],
            object_types: vec![RawObjectType {
                object_type: "limit_order".to_string(),
                cpp_alias: "limit_order_id_type".to_string(),
                object_space_name: "protocol_ids".to_string(),
                object_space: Some(1),
                object_type_name: "limit_order_object_type".to_string(),
                type_id: Some(7),
                struct_ref: Some("limit_order_object".to_string()),
                source: SourceLoc {
                    file: PathBuf::from("types.hpp"),
                    line: 10,
                },
                id_namespace: "protocol".to_string(),
            }],
            ..SourceFacts::default()
        };
        let rpc_methods = vec![RpcMethodDef {
            name: "get_limit_orders".to_string(),
            api_class: "database_api".to_string(),
            api_name: Some("database".to_string()),
            params: vec![],
            returns: Some(TypeRef::Vector {
                inner: Box::new(TypeRef::Ref {
                    name: "limit_order_object".to_string(),
                }),
            }),
            is_subscription: false,
            notices: vec![],
            binding_hints: None,
            source: None,
            support: None,
        }];

        let protocol = build_protocol(&config, &facts, rpc_methods);

        assert_eq!(
            protocol.object_types[0].struct_ref.as_deref(),
            Some("limit_order_object")
        );
        let limit_order = protocol
            .structs
            .iter()
            .find(|struct_def| struct_def.name == "limit_order_object")
            .expect("limit_order_object emitted");
        assert_eq!(limit_order.fields[0].name, "id");
        assert_eq!(
            limit_order.fields[0].ty,
            TypeRef::ProtocolObjectId {
                object_type: "limit_order".to_string()
            }
        );
        assert_eq!(limit_order.fields[1].name, "seller");
        assert_eq!(limit_order.fields[1].index, 1);
    }

    #[test]
    fn build_protocol_seeds_configured_object_structs() {
        let config: GeneratorConfig = toml::from_str(
            r#"
            object_structs = ["asset_object"]

            [chain]
            id = "bitshares"
            public_key_prefix = "BTS"

            [output]
            dist = "./dist/bitshares.open-graphene.json"

            [source]
            chain_repo = "../../blockchains/bitshares/bitshares-core"
            "#,
        )
        .expect("parse config");
        let facts = SourceFacts {
            classes: vec![RawClass {
                name: "asset_object".to_string(),
                qualified_name: Some("graphene::chain::asset_object".to_string()),
                methods: vec![],
                fields: vec![RawField {
                    name: "symbol".to_string(),
                    type_expr: "string".to_string(),
                    source: SourceLoc {
                        file: PathBuf::from("asset_object.hpp"),
                        line: 80,
                    },
                }],
                source: SourceLoc {
                    file: PathBuf::from("asset_object.hpp"),
                    line: 75,
                },
            }],
            object_types: vec![RawObjectType {
                object_type: "asset".to_string(),
                cpp_alias: "asset_id_type".to_string(),
                object_space_name: "protocol_ids".to_string(),
                object_space: Some(1),
                object_type_name: "asset_object_type".to_string(),
                type_id: Some(3),
                struct_ref: Some("asset_object".to_string()),
                source: SourceLoc {
                    file: PathBuf::from("types.hpp"),
                    line: 10,
                },
                id_namespace: "protocol".to_string(),
            }],
            ..SourceFacts::default()
        };

        let protocol = build_protocol(&config, &facts, vec![]);

        let asset = protocol
            .structs
            .iter()
            .find(|struct_def| struct_def.name == "asset_object")
            .expect("configured asset_object emitted");
        assert_eq!(asset.fields[0].name, "id");
        assert_eq!(asset.fields[1].name, "symbol");
        assert_eq!(
            protocol.object_types[0].struct_ref.as_deref(),
            Some("asset_object")
        );
    }

    #[test]
    fn build_protocol_emits_operations_from_operation_static_variant() {
        let config: GeneratorConfig = toml::from_str(
            r#"
            [chain]
            id = "bitshares"
            public_key_prefix = "BTS"

            [output]
            dist = "./dist/bitshares.open-graphene.json"

            [source]
            chain_repo = "../../blockchains/bitshares/bitshares-core"
            "#,
        )
        .expect("parse config");
        let facts = SourceFacts {
            classes: vec![RawClass {
                name: "transfer_operation".to_string(),
                qualified_name: Some("graphene::protocol::transfer_operation".to_string()),
                methods: vec![],
                fields: vec![RawField {
                    name: "fee".to_string(),
                    type_expr: "asset".to_string(),
                    source: SourceLoc {
                        file: PathBuf::from("transfer.hpp"),
                        line: 11,
                    },
                }],
                source: SourceLoc {
                    file: PathBuf::from("transfer.hpp"),
                    line: 10,
                },
            }],
            static_variants: vec![RawStaticVariant {
                name: "operation".to_string(),
                variants: vec!["transfer_operation".to_string()],
                source: SourceLoc {
                    file: PathBuf::from("operations.hpp"),
                    line: 20,
                },
            }],
            ..SourceFacts::default()
        };
        let rpc_methods = vec![RpcMethodDef {
            name: "get_op".to_string(),
            api_class: "database_api".to_string(),
            api_name: Some("database".to_string()),
            params: vec![],
            returns: Some(TypeRef::StaticVariantRef {
                name: "operation".to_string(),
            }),
            is_subscription: false,
            notices: vec![],
            binding_hints: None,
            source: None,
            support: None,
        }];

        let protocol = build_protocol(&config, &facts, rpc_methods);

        assert_eq!(protocol.operations.len(), 1);
        assert_eq!(protocol.operations[0].name, "transfer_operation");
        assert_eq!(protocol.operations[0].wire_tag, 0);
        assert_eq!(protocol.operations[0].fields.len(), 1);
        assert_eq!(protocol.operations[0].fields[0].name, "fee");
    }

    #[test]
    fn build_protocol_seeds_base_transaction_struct() {
        let config: GeneratorConfig = toml::from_str(
            r#"
            [chain]
            id = "bitshares"
            public_key_prefix = "BTS"

            [output]
            dist = "./dist/bitshares.open-graphene.json"

            [source]
            chain_repo = "../../blockchains/bitshares/bitshares-core"
            "#,
        )
        .expect("parse config");
        let facts = SourceFacts {
            classes: vec![
                RawClass {
                    name: "transaction".to_string(),
                    qualified_name: Some("graphene::protocol::transaction".to_string()),
                    methods: vec![],
                    fields: vec![
                        RawField {
                            name: "ref_block_num".to_string(),
                            type_expr: "uint16_t".to_string(),
                            source: SourceLoc {
                                file: PathBuf::from("transaction.hpp"),
                                line: 78,
                            },
                        },
                        RawField {
                            name: "ref_block_prefix".to_string(),
                            type_expr: "uint32_t".to_string(),
                            source: SourceLoc {
                                file: PathBuf::from("transaction.hpp"),
                                line: 84,
                            },
                        },
                        RawField {
                            name: "expiration".to_string(),
                            type_expr: "fc::time_point_sec".to_string(),
                            source: SourceLoc {
                                file: PathBuf::from("transaction.hpp"),
                                line: 91,
                            },
                        },
                        RawField {
                            name: "operations".to_string(),
                            type_expr: "vector<operation>".to_string(),
                            source: SourceLoc {
                                file: PathBuf::from("transaction.hpp"),
                                line: 93,
                            },
                        },
                        RawField {
                            name: "extensions".to_string(),
                            type_expr: "extensions_type".to_string(),
                            source: SourceLoc {
                                file: PathBuf::from("transaction.hpp"),
                                line: 94,
                            },
                        },
                    ],
                    source: SourceLoc {
                        file: PathBuf::from("transaction.hpp"),
                        line: 69,
                    },
                },
                RawClass {
                    name: "signed_transaction".to_string(),
                    qualified_name: Some("graphene::protocol::signed_transaction".to_string()),
                    methods: vec![],
                    fields: vec![RawField {
                        name: "signatures".to_string(),
                        type_expr: "vector<signature_type>".to_string(),
                        source: SourceLoc {
                            file: PathBuf::from("transaction.hpp"),
                            line: 217,
                        },
                    }],
                    source: SourceLoc {
                        file: PathBuf::from("transaction.hpp"),
                        line: 203,
                    },
                },
            ],
            reflects: vec![
                RawReflect {
                    type_name: "graphene::protocol::transaction".to_string(),
                    bases: vec![],
                    fields: vec![
                        "ref_block_num".to_string(),
                        "ref_block_prefix".to_string(),
                        "expiration".to_string(),
                        "operations".to_string(),
                        "extensions".to_string(),
                    ],
                    source: SourceLoc {
                        file: PathBuf::from("transaction.hpp"),
                        line: 307,
                    },
                    derived: false,
                },
                RawReflect {
                    type_name: "graphene::protocol::signed_transaction".to_string(),
                    bases: vec!["graphene::protocol::transaction".to_string()],
                    fields: vec!["signatures".to_string()],
                    source: SourceLoc {
                        file: PathBuf::from("transaction.hpp"),
                        line: 310,
                    },
                    derived: true,
                },
            ],
            static_variants: vec![
                RawStaticVariant {
                    name: "operation".to_string(),
                    variants: vec!["transfer_operation".to_string()],
                    source: SourceLoc {
                        file: PathBuf::from("operations.hpp"),
                        line: 70,
                    },
                },
                RawStaticVariant {
                    name: "future_extensions".to_string(),
                    variants: vec!["void_t".to_string()],
                    source: SourceLoc {
                        file: PathBuf::from("types.hpp"),
                        line: 30,
                    },
                },
            ],
            ..SourceFacts::default()
        };

        let protocol = build_protocol(&config, &facts, vec![]);
        let transaction = protocol
            .structs
            .iter()
            .find(|struct_def| struct_def.name == "transaction")
            .expect("transaction emitted");

        assert_eq!(
            transaction
                .fields
                .iter()
                .map(|field| field.name.as_str())
                .collect::<Vec<_>>(),
            vec![
                "ref_block_num",
                "ref_block_prefix",
                "expiration",
                "operations",
                "extensions"
            ]
        );
        assert_eq!(transaction.fields[0].ty, TypeRef::Uint16);
        assert_eq!(transaction.fields[1].ty, TypeRef::Uint32);
        assert_eq!(transaction.fields[2].ty, TypeRef::TimePointSec);
        assert_eq!(
            transaction.fields[3].ty,
            TypeRef::Vector {
                inner: Box::new(TypeRef::StaticVariantRef {
                    name: "operation".to_string()
                })
            }
        );
        assert_eq!(
            transaction.fields[4].ty,
            TypeRef::Set {
                inner: Box::new(TypeRef::StaticVariantRef {
                    name: "future_extensions".to_string()
                }),
                ordering: OrderingRule::StaticVariantTag,
            }
        );
        assert!(
            protocol
                .static_variants
                .iter()
                .any(|variant| variant.name == "operation")
        );
        assert!(
            protocol
                .static_variants
                .iter()
                .any(|variant| variant.name == "future_extensions")
        );
        let signed_transaction = protocol
            .structs
            .iter()
            .find(|struct_def| struct_def.name == "signed_transaction")
            .expect("signed_transaction emitted");
        assert_eq!(
            signed_transaction
                .fields
                .iter()
                .map(|field| field.name.as_str())
                .collect::<Vec<_>>(),
            vec![
                "ref_block_num",
                "ref_block_prefix",
                "expiration",
                "operations",
                "extensions",
                "signatures"
            ]
        );
        assert_eq!(
            signed_transaction.fields[5].ty,
            TypeRef::Vector {
                inner: Box::new(TypeRef::Signature)
            }
        );
    }

    #[test]
    fn build_protocol_includes_derived_reflect_base_fields() {
        let config: GeneratorConfig = toml::from_str(
            r#"
            [chain]
            id = "bitshares"
            public_key_prefix = "BTS"

            [output]
            dist = "./dist/bitshares.open-graphene.json"

            [source]
            chain_repo = "../../blockchains/bitshares/bitshares-core"
            "#,
        )
        .expect("parse config");
        let facts = SourceFacts {
            classes: vec![
                RawClass {
                    name: "block_header".to_string(),
                    qualified_name: Some("graphene::protocol::block_header".to_string()),
                    methods: vec![],
                    fields: vec![RawField {
                        name: "previous".to_string(),
                        type_expr: "block_id_type".to_string(),
                        source: SourceLoc {
                            file: PathBuf::from("block.hpp"),
                            line: 10,
                        },
                    }],
                    source: SourceLoc {
                        file: PathBuf::from("block.hpp"),
                        line: 9,
                    },
                },
                RawClass {
                    name: "maybe_signed_block_header".to_string(),
                    qualified_name: Some("graphene::app::maybe_signed_block_header".to_string()),
                    methods: vec![],
                    fields: vec![RawField {
                        name: "witness_signature".to_string(),
                        type_expr: "optional<signature_type>".to_string(),
                        source: SourceLoc {
                            file: PathBuf::from("api_objects.hpp"),
                            line: 20,
                        },
                    }],
                    source: SourceLoc {
                        file: PathBuf::from("api_objects.hpp"),
                        line: 18,
                    },
                },
            ],
            reflects: vec![
                RawReflect {
                    type_name: "graphene::protocol::block_header".to_string(),
                    bases: vec![],
                    fields: vec!["previous".to_string()],
                    source: SourceLoc {
                        file: PathBuf::from("block.hpp"),
                        line: 50,
                    },
                    derived: false,
                },
                RawReflect {
                    type_name: "graphene::app::maybe_signed_block_header".to_string(),
                    bases: vec!["graphene::protocol::block_header".to_string()],
                    fields: vec!["witness_signature".to_string()],
                    source: SourceLoc {
                        file: PathBuf::from("api_objects.hpp"),
                        line: 60,
                    },
                    derived: true,
                },
            ],
            ..SourceFacts::default()
        };
        let rpc_methods = vec![RpcMethodDef {
            name: "get_block_header".to_string(),
            api_class: "database_api".to_string(),
            api_name: Some("database".to_string()),
            params: vec![],
            returns: Some(TypeRef::Ref {
                name: "maybe_signed_block_header".to_string(),
            }),
            is_subscription: false,
            notices: vec![],
            binding_hints: None,
            source: None,
            support: None,
        }];

        let protocol = build_protocol(&config, &facts, rpc_methods);
        let header = protocol
            .structs
            .iter()
            .find(|struct_def| struct_def.name == "maybe_signed_block_header")
            .expect("maybe_signed_block_header emitted");
        assert_eq!(
            header
                .fields
                .iter()
                .map(|field| field.name.as_str())
                .collect::<Vec<_>>(),
            vec!["previous", "witness_signature"]
        );
    }

    #[test]
    fn build_protocol_synthesizes_fee_parameters_static_variant() {
        let config: GeneratorConfig = toml::from_str(
            r#"
            [chain]
            id = "bitshares"
            public_key_prefix = "BTS"

            [output]
            dist = "./dist/bitshares.open-graphene.json"

            [source]
            chain_repo = "../../blockchains/bitshares/bitshares-core"
            "#,
        )
        .expect("parse config");
        let facts = SourceFacts {
            classes: vec![
                RawClass {
                    name: "fee_schedule".to_string(),
                    qualified_name: Some("graphene::protocol::fee_schedule".to_string()),
                    methods: vec![],
                    fields: vec![RawField {
                        name: "parameters".to_string(),
                        type_expr: "fee_parameters::flat_set_type".to_string(),
                        source: SourceLoc {
                            file: PathBuf::from("fee_schedule.hpp"),
                            line: 20,
                        },
                    }],
                    source: SourceLoc {
                        file: PathBuf::from("fee_schedule.hpp"),
                        line: 18,
                    },
                },
                RawClass {
                    name: "transfer_operation".to_string(),
                    qualified_name: Some("graphene::protocol::transfer_operation".to_string()),
                    methods: vec![],
                    fields: vec![],
                    source: SourceLoc {
                        file: PathBuf::from("transfer.hpp"),
                        line: 10,
                    },
                },
                RawClass {
                    name: "fee_params_t".to_string(),
                    qualified_name: None,
                    methods: vec![],
                    fields: vec![RawField {
                        name: "fee".to_string(),
                        type_expr: "uint64_t".to_string(),
                        source: SourceLoc {
                            file: PathBuf::from("transfer.hpp"),
                            line: 12,
                        },
                    }],
                    source: SourceLoc {
                        file: PathBuf::from("transfer.hpp"),
                        line: 11,
                    },
                },
            ],
            static_variants: vec![RawStaticVariant {
                name: "operation".to_string(),
                variants: vec!["transfer_operation".to_string()],
                source: SourceLoc {
                    file: PathBuf::from("operations.hpp"),
                    line: 30,
                },
            }],
            reflects: vec![
                RawReflect {
                    type_name: "graphene::protocol::fee_schedule".to_string(),
                    bases: vec![],
                    fields: vec!["parameters".to_string()],
                    source: SourceLoc {
                        file: PathBuf::from("fee_schedule.hpp"),
                        line: 40,
                    },
                    derived: false,
                },
                RawReflect {
                    type_name: "graphene::protocol::transfer_operation::fee_params_t".to_string(),
                    bases: vec![],
                    fields: vec!["fee".to_string()],
                    source: SourceLoc {
                        file: PathBuf::from("transfer.hpp"),
                        line: 41,
                    },
                    derived: false,
                },
            ],
            ..SourceFacts::default()
        };
        let rpc_methods = vec![RpcMethodDef {
            name: "get_fee_schedule".to_string(),
            api_class: "database_api".to_string(),
            api_name: Some("database".to_string()),
            params: vec![],
            returns: Some(TypeRef::Ref {
                name: "fee_schedule".to_string(),
            }),
            is_subscription: false,
            notices: vec![],
            binding_hints: None,
            source: None,
            support: None,
        }];

        let protocol = build_protocol(&config, &facts, rpc_methods);
        let fee_schedule = protocol
            .structs
            .iter()
            .find(|struct_def| struct_def.name == "fee_schedule")
            .expect("fee_schedule emitted");
        assert_eq!(
            fee_schedule.fields[0].ty,
            TypeRef::Set {
                inner: Box::new(TypeRef::StaticVariantRef {
                    name: "fee_parameters".to_string()
                }),
                ordering: open_graphene_json_schema::types::OrderingRule::StaticVariantTag,
            }
        );
        let fee_parameters = protocol
            .static_variants
            .iter()
            .find(|variant| variant.name == "fee_parameters")
            .expect("fee_parameters emitted");
        assert_eq!(fee_parameters.variants[0].tag, 0);
        assert_eq!(
            fee_parameters.variants[0].name,
            "transfer_operation_fee_params_t"
        );
        assert_eq!(
            fee_parameters.variants[0].ty,
            TypeRef::Ref {
                name: "transfer_operation_fee_params_t".to_string()
            }
        );
        let fee_params = protocol
            .structs
            .iter()
            .find(|struct_def| struct_def.name == "transfer_operation_fee_params_t")
            .expect("fee_params_t emitted");
        assert_eq!(fee_params.fields[0].name, "fee");
    }

    #[test]
    fn build_protocol_resolves_nested_extension_payload_structs() {
        let config: GeneratorConfig = toml::from_str(
            r#"
            [chain]
            id = "bitshares"
            public_key_prefix = "BTS"

            [output]
            dist = "./dist/bitshares.open-graphene.json"

            [source]
            chain_repo = "../../blockchains/bitshares/bitshares-core"
            "#,
        )
        .expect("parse config");
        let source = SourceLoc {
            file: PathBuf::from("account.hpp"),
            line: 10,
        };
        let facts = SourceFacts {
            classes: vec![
                RawClass {
                    name: "account_create_operation".to_string(),
                    qualified_name: Some(
                        "graphene::protocol::account_create_operation".to_string(),
                    ),
                    methods: vec![],
                    fields: vec![RawField {
                        name: "extensions".to_string(),
                        type_expr: "extension<ext>".to_string(),
                        source: source.clone(),
                    }],
                    source: source.clone(),
                },
                RawClass {
                    name: "ext".to_string(),
                    qualified_name: None,
                    methods: vec![],
                    fields: vec![RawField {
                        name: "owner_special_authority".to_string(),
                        type_expr: "special_authority".to_string(),
                        source: SourceLoc {
                            file: PathBuf::from("account.hpp"),
                            line: 20,
                        },
                    }],
                    source: SourceLoc {
                        file: PathBuf::from("account.hpp"),
                        line: 19,
                    },
                },
            ],
            static_variants: vec![RawStaticVariant {
                name: "operation".to_string(),
                variants: vec!["account_create_operation".to_string()],
                source: SourceLoc {
                    file: PathBuf::from("operations.hpp"),
                    line: 30,
                },
            }],
            reflects: vec![
                RawReflect {
                    type_name: "graphene::protocol::account_create_operation".to_string(),
                    bases: vec![],
                    fields: vec!["extensions".to_string()],
                    source: SourceLoc {
                        file: PathBuf::from("account.hpp"),
                        line: 40,
                    },
                    derived: false,
                },
                RawReflect {
                    type_name: "graphene::protocol::account_create_operation::ext".to_string(),
                    bases: vec![],
                    fields: vec!["owner_special_authority".to_string()],
                    source: SourceLoc {
                        file: PathBuf::from("account.hpp"),
                        line: 41,
                    },
                    derived: false,
                },
            ],
            ..SourceFacts::default()
        };
        let rpc_methods = vec![RpcMethodDef {
            name: "get_op".to_string(),
            api_class: "database_api".to_string(),
            api_name: Some("database".to_string()),
            params: vec![],
            returns: Some(TypeRef::StaticVariantRef {
                name: "operation".to_string(),
            }),
            is_subscription: false,
            notices: vec![],
            binding_hints: None,
            source: None,
            support: None,
        }];

        let protocol = build_protocol(&config, &facts, rpc_methods);
        let parent = protocol
            .structs
            .iter()
            .find(|struct_def| struct_def.name == "account_create_operation")
            .expect("parent struct emitted");
        assert_eq!(
            parent.fields[0].ty,
            TypeRef::Ref {
                name: "account_create_operation_ext".to_string()
            }
        );
        let nested = protocol
            .structs
            .iter()
            .find(|struct_def| struct_def.name == "account_create_operation_ext")
            .expect("nested extension struct emitted");
        assert_eq!(
            nested.source_name.as_deref(),
            Some("graphene::protocol::account_create_operation::ext")
        );
        assert_eq!(nested.fields[0].name, "owner_special_authority");
    }

    #[test]
    fn build_protocol_uses_reflect_order_and_omits_unreflected_fields() {
        let config: GeneratorConfig = toml::from_str(
            r#"
            [chain]
            id = "bitshares"
            public_key_prefix = "BTS"

            [output]
            dist = "./dist/bitshares.open-graphene.json"

            [source]
            chain_repo = "../../blockchains/bitshares/bitshares-core"
            "#,
        )
        .expect("parse config");
        let facts = SourceFacts {
            classes: vec![RawClass {
                name: "sample_operation".to_string(),
                qualified_name: Some("graphene::protocol::sample_operation".to_string()),
                methods: vec![],
                fields: vec![
                    RawField {
                        name: "b".to_string(),
                        type_expr: "uint32_t".to_string(),
                        source: SourceLoc {
                            file: PathBuf::from("sample.hpp"),
                            line: 11,
                        },
                    },
                    RawField {
                        name: "internal_cache".to_string(),
                        type_expr: "uint32_t".to_string(),
                        source: SourceLoc {
                            file: PathBuf::from("sample.hpp"),
                            line: 12,
                        },
                    },
                    RawField {
                        name: "a".to_string(),
                        type_expr: "bool".to_string(),
                        source: SourceLoc {
                            file: PathBuf::from("sample.hpp"),
                            line: 13,
                        },
                    },
                ],
                source: SourceLoc {
                    file: PathBuf::from("sample.hpp"),
                    line: 10,
                },
            }],
            static_variants: vec![RawStaticVariant {
                name: "operation".to_string(),
                variants: vec!["sample_operation".to_string()],
                source: SourceLoc {
                    file: PathBuf::from("operations.hpp"),
                    line: 20,
                },
            }],
            reflects: vec![RawReflect {
                type_name: "graphene::protocol::sample_operation".to_string(),
                bases: vec![],
                fields: vec!["a".to_string(), "b".to_string()],
                source: SourceLoc {
                    file: PathBuf::from("sample.hpp"),
                    line: 30,
                },
                derived: false,
            }],
            ..SourceFacts::default()
        };
        let rpc_methods = vec![RpcMethodDef {
            name: "get_op".to_string(),
            api_class: "database_api".to_string(),
            api_name: Some("database".to_string()),
            params: vec![],
            returns: Some(TypeRef::StaticVariantRef {
                name: "operation".to_string(),
            }),
            is_subscription: false,
            notices: vec![],
            binding_hints: None,
            source: None,
            support: None,
        }];

        let protocol = build_protocol(&config, &facts, rpc_methods);

        let sample = protocol
            .structs
            .iter()
            .find(|struct_def| struct_def.name == "sample_operation")
            .expect("sample struct emitted");
        assert_eq!(
            sample
                .fields
                .iter()
                .map(|field| field.name.as_str())
                .collect::<Vec<_>>(),
            vec!["a", "b"]
        );
        assert_eq!(protocol.operations[0].fields[0].name, "a");
        assert_eq!(protocol.operations[0].fields[1].name, "b");
    }
}
