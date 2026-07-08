use super::*;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct SchemaFieldKey {
    pub(crate) owner: String,
    pub(crate) field: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct SchemaVariantKey {
    pub(crate) owner: String,
    pub(crate) variant: String,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct SchemaNoRecursionCuts {
    pub(crate) fields: BTreeSet<SchemaFieldKey>,
    pub(crate) variants: BTreeSet<SchemaVariantKey>,
}

#[derive(Debug, Clone)]
pub(crate) enum SchemaCut {
    Field(SchemaFieldKey),
    Variant(SchemaVariantKey),
}

#[derive(Debug, Clone)]
pub(crate) struct SchemaEdge {
    target: String,
    cut: Option<SchemaCut>,
}

pub(crate) fn detect_schema_no_recursion_cuts(protocol: &Protocol) -> SchemaNoRecursionCuts {
    let graph = build_schema_dependency_graph(protocol);
    let mut cuts = SchemaNoRecursionCuts::default();
    let mut visiting = BTreeSet::new();
    let mut path = Vec::new();

    for node in graph.keys() {
        detect_schema_cycles_from(node, &graph, &mut visiting, &mut path, &mut cuts);
    }

    cuts.variants.retain(|variant| variant.owner != "Operation");
    cuts
}

pub(crate) fn build_schema_dependency_graph(
    protocol: &Protocol,
) -> BTreeMap<String, Vec<SchemaEdge>> {
    let mut graph = BTreeMap::new();

    for struct_def in &protocol.structs {
        if struct_def.kind == StructKind::Operation || is_operation_ref(protocol, &struct_def.name)
        {
            continue;
        }
        let owner = rust_type_name(&struct_def.name);
        graph.entry(owner.clone()).or_insert_with(Vec::new);
        for field in &struct_def.fields {
            add_schema_field_edges(protocol, &mut graph, &owner, field);
        }
    }

    for operation in &protocol.operations {
        let owner = rust_type_name(&operation.name);
        graph.entry(owner.clone()).or_insert_with(Vec::new);
        for field in &operation.fields {
            add_schema_field_edges(protocol, &mut graph, &owner, field);
        }
    }

    for variant in &protocol.static_variants {
        let owner = rust_type_name(&variant.name);
        graph.entry(owner.clone()).or_insert_with(Vec::new);
        for arm in &variant.variants {
            let variant_key = SchemaVariantKey {
                owner: owner.clone(),
                variant: rust_variant_name(&arm.name),
            };
            for target in schema_type_targets(protocol, &arm.ty) {
                graph
                    .entry(owner.clone())
                    .or_insert_with(Vec::new)
                    .push(SchemaEdge {
                        target,
                        cut: Some(SchemaCut::Variant(variant_key.clone())),
                    });
            }
        }
    }

    graph
}

pub(crate) fn add_schema_field_edges(
    protocol: &Protocol,
    graph: &mut BTreeMap<String, Vec<SchemaEdge>>,
    owner: &str,
    field: &FieldDef,
) {
    let field_key = SchemaFieldKey {
        owner: owner.to_string(),
        field: rust_field_name(&field.name),
    };
    for target in schema_type_targets(protocol, &field.ty) {
        graph
            .entry(owner.to_string())
            .or_default()
            .push(SchemaEdge {
                target,
                cut: Some(SchemaCut::Field(field_key.clone())),
            });
    }
}

pub(crate) fn schema_type_targets(protocol: &Protocol, ty: &TypeRef) -> BTreeSet<String> {
    let mut targets = BTreeSet::new();
    collect_schema_type_targets(protocol, ty, &mut targets);
    targets
}

pub(crate) fn collect_schema_type_targets(
    protocol: &Protocol,
    ty: &TypeRef,
    targets: &mut BTreeSet<String>,
) {
    match ty {
        TypeRef::Optional { inner } | TypeRef::Vector { inner } | TypeRef::Set { inner, .. } => {
            collect_schema_type_targets(protocol, inner, targets)
        }
        TypeRef::Map { key, value, .. } | TypeRef::FlatMap { key, value, .. } => {
            collect_schema_type_targets(protocol, key, targets);
            collect_schema_type_targets(protocol, value, targets);
        }
        TypeRef::Pair { first, second } => {
            collect_schema_type_targets(protocol, first, targets);
            collect_schema_type_targets(protocol, second, targets);
        }
        TypeRef::Ref { name } if is_operation_ref(protocol, name) => {
            targets.insert(rust_type_name(name));
        }
        TypeRef::Ref { name } | TypeRef::StaticVariantRef { name } => {
            targets.insert(rust_type_name(name));
        }
        TypeRef::Void
        | TypeRef::Bool
        | TypeRef::Uint8
        | TypeRef::Uint16
        | TypeRef::Uint32
        | TypeRef::Int32 { .. }
        | TypeRef::Int64 { .. }
        | TypeRef::Uint64 { .. }
        | TypeRef::Uint128 { .. }
        | TypeRef::UnsignedVarint
        | TypeRef::CallbackHandle
        | TypeRef::String
        | TypeRef::Bytes
        | TypeRef::FixedHex { .. }
        | TypeRef::FixedBytes { .. }
        | TypeRef::TimePointSec
        | TypeRef::TimePoint
        | TypeRef::PublicKey { .. }
        | TypeRef::Address
        | TypeRef::Signature
        | TypeRef::ObjectId
        | TypeRef::ProtocolObjectId { .. }
        | TypeRef::ProtocolObjectUnion { .. }
        | TypeRef::VoteId
        | TypeRef::AnyJson { .. }
        | TypeRef::Unsupported { .. } => {}
    }
}

pub(crate) fn detect_schema_cycles_from(
    node: &str,
    graph: &BTreeMap<String, Vec<SchemaEdge>>,
    visiting: &mut BTreeSet<String>,
    path: &mut Vec<Option<SchemaCut>>,
    cuts: &mut SchemaNoRecursionCuts,
) {
    if !visiting.insert(node.to_string()) {
        return;
    }

    if let Some(edges) = graph.get(node) {
        for edge in edges {
            if visiting.contains(&edge.target) {
                if let Some(cut) = choose_schema_cycle_cut(&edge.target, &edge.cut, path) {
                    record_schema_cycle_cut(cut, cuts);
                }
                continue;
            }

            path.push(edge.cut.clone());
            detect_schema_cycles_from(&edge.target, graph, visiting, path, cuts);
            path.pop();
        }
    }

    visiting.remove(node);
}

pub(crate) fn choose_schema_cycle_cut(
    target: &str,
    back_edge_cut: &Option<SchemaCut>,
    path: &[Option<SchemaCut>],
) -> Option<SchemaCut> {
    if let Some(SchemaCut::Variant(variant)) = back_edge_cut {
        if variant.owner != "Operation" {
            return Some(SchemaCut::Variant(variant.clone()));
        }
        if let Some(field) = path.iter().rev().find_map(|cut| match cut {
            Some(SchemaCut::Field(field)) => Some(field.clone()),
            _ => None,
        }) {
            return Some(SchemaCut::Field(field));
        }
    }

    if let Some(field) = path.iter().rev().find_map(|cut| match cut {
        Some(SchemaCut::Field(field)) if field.owner == target => Some(field.clone()),
        _ => None,
    }) {
        return Some(SchemaCut::Field(field));
    }

    if let Some(variant) = path.iter().rev().find_map(|cut| match cut {
        Some(SchemaCut::Variant(variant)) => Some(variant.clone()),
        _ => None,
    }) {
        return Some(SchemaCut::Variant(variant));
    }

    back_edge_cut
        .clone()
        .or_else(|| path.iter().rev().find_map(|cut| cut.clone()))
}

pub(crate) fn record_schema_cycle_cut(cut: SchemaCut, cuts: &mut SchemaNoRecursionCuts) {
    match cut {
        SchemaCut::Field(field) => {
            cuts.fields.insert(field);
        }
        SchemaCut::Variant(variant) => {
            cuts.variants.insert(variant);
        }
    }
}
