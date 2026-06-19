use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use open_graphene_json_schema::{
    EnumDef, FieldDef, JsonShape, OperationDef, Protocol, StaticVariantDef, StructDef, StructKind,
    TypeRef,
};

use crate::error::{GenBindingsRsError, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerateBindingsResult {
    pub output_path: PathBuf,
    pub chain_id: String,
    pub schema_version: u32,
    pub rpc_method_count: usize,
    pub struct_count: usize,
    pub operation_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct SchemaFieldKey {
    owner: String,
    field: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct SchemaVariantKey {
    owner: String,
    variant: String,
}

#[derive(Debug, Clone, Default)]
struct SchemaNoRecursionCuts {
    fields: BTreeSet<SchemaFieldKey>,
    variants: BTreeSet<SchemaVariantKey>,
}

#[derive(Debug, Clone)]
enum SchemaCut {
    Field(SchemaFieldKey),
    Variant(SchemaVariantKey),
}

#[derive(Debug, Clone)]
struct SchemaEdge {
    target: String,
    cut: Option<SchemaCut>,
}

#[derive(Debug, Clone)]
struct DfsEdge {
    cut: Option<SchemaCut>,
}

pub fn generate_bindings(
    spec_path: impl AsRef<Path>,
    out_dir: impl AsRef<Path>,
) -> Result<GenerateBindingsResult> {
    let spec_path = spec_path.as_ref();
    let out_dir = out_dir.as_ref();

    let spec_json =
        fs::read_to_string(spec_path).map_err(|source| GenBindingsRsError::ReadSpec {
            path: spec_path.to_path_buf(),
            source,
        })?;
    let protocol: Protocol =
        serde_json::from_str(&spec_json).map_err(|source| GenBindingsRsError::ParseSpec {
            path: spec_path.to_path_buf(),
            source,
        })?;

    fs::create_dir_all(out_dir).map_err(|source| GenBindingsRsError::CreateOutDir {
        path: out_dir.to_path_buf(),
        source,
    })?;

    let files = [
        ("mod.rs", render_mod(&protocol)?),
        ("ids.rs", render_ids(&protocol)?),
        ("types.rs", render_types(&protocol)?),
        ("operations.rs", render_operations(&protocol)?),
        ("static_variants.rs", render_static_variants(&protocol)?),
        ("fc.rs", render_fc(&protocol)?),
    ];

    for (file_name, contents) in files {
        let path = out_dir.join(file_name);
        fs::write(&path, normalize_generated_contents(&contents))
            .map_err(|source| GenBindingsRsError::WriteOutput { path, source })?;
    }

    Ok(GenerateBindingsResult {
        output_path: out_dir.join("mod.rs"),
        chain_id: protocol.chain.id,
        schema_version: protocol.schema_version,
        rpc_method_count: protocol.rpc_methods.len(),
        struct_count: protocol.structs.len(),
        operation_count: protocol.operations.len(),
    })
}

fn normalize_generated_contents(contents: &str) -> String {
    format!("{}\n", contents.trim_end())
}

fn render_mod(protocol: &Protocol) -> Result<String> {
    let mut out = generated_header(protocol, "module index");
    out.push_str("pub mod fc;\n");
    out.push_str("pub mod ids;\n");
    out.push_str("pub mod operations;\n");
    out.push_str("pub mod static_variants;\n");
    out.push_str("pub mod types;\n\n");
    out.push_str("pub use fc::*;\n");
    out.push_str("pub use ids::*;\n");
    out.push_str("pub use operations::*;\n");
    out.push_str("pub use static_variants::*;\n");
    out.push_str("pub use types::*;\n");
    Ok(out)
}

fn render_ids(protocol: &Protocol) -> Result<String> {
    let mut out = generated_header(protocol, "metadata and protocol object ids");
    out.push_str("use serde::{Deserialize, Serialize};\n\n");
    out.push_str(&format!(
        "pub const SCHEMA_VERSION: u32 = {};\n",
        protocol.schema_version
    ));
    out.push_str(&format!(
        "pub const CHAIN_ID: &str = {};\n",
        rust_string_literal(&protocol.chain.id)
    ));
    if let Some(chain_id) = &protocol.chain.chain_id {
        out.push_str(&format!(
            "pub const CHAIN_ID_HEX: &str = {};\n",
            rust_string_literal(chain_id)
        ));
    }
    out.push_str(&format!(
        "pub const PUBLIC_KEY_PREFIX: &str = {};\n\n",
        rust_string_literal(&protocol.chain.public_key_prefix)
    ));
    out.push_str(&format!(
        "pub const STRUCT_COUNT: usize = {};\n",
        protocol.structs.len()
    ));
    out.push_str(&format!(
        "pub const ENUM_COUNT: usize = {};\n",
        protocol.enums.len()
    ));
    out.push_str(&format!(
        "pub const STATIC_VARIANT_COUNT: usize = {};\n",
        protocol.static_variants.len()
    ));
    out.push_str(&format!(
        "pub const OPERATION_COUNT: usize = {};\n",
        protocol.operations.len()
    ));
    out.push_str(&format!(
        "pub const OBJECT_TYPE_COUNT: usize = {};\n",
        protocol.object_types.len()
    ));
    out.push_str(&format!(
        "pub const RPC_API_COUNT: usize = {};\n",
        protocol.rpc_apis.len()
    ));
    out.push_str(&format!(
        "pub const RPC_METHOD_COUNT: usize = {};\n\n",
        protocol.rpc_methods.len()
    ));

    out.push_str(
        "#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, utoipa::ToSchema)]\n",
    );
    out.push_str(&format!(
        "#[schema(as = {})]\n",
        openapi_schema_name(protocol, "ObjectId")
    ));
    out.push_str("#[serde(transparent)]\n");
    out.push_str("pub struct ObjectId(pub String);\n\n");
    render_string_id_conversions(&mut out, "ObjectId");

    let mut emitted = BTreeSet::new();
    let mut protocol_object_ids = collect_protocol_object_id_names(protocol);
    protocol_object_ids.extend(
        protocol
            .object_types
            .iter()
            .map(|object_type| object_type.object_type.clone()),
    );

    for object_type_name in protocol_object_ids {
        let id_name = object_id_type_name(&object_type_name);
        if !emitted.insert(id_name.clone()) {
            return Err(GenBindingsRsError::Render {
                message: format!("duplicate generated object id type `{id_name}`"),
            });
        }

        let object_type = protocol
            .object_types
            .iter()
            .find(|object_type| object_type.object_type == object_type_name);
        out.push_str(&format!(
            "/// Object ID for `{}` protocol objects.\n",
            object_type_name
        ));
        out.push_str("#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, utoipa::ToSchema)]\n");
        out.push_str(&format!(
            "#[schema(as = {})]\n",
            openapi_schema_name(protocol, &id_name)
        ));
        out.push_str("#[serde(transparent)]\n");
        out.push_str(&format!("pub struct {id_name}(pub String);\n\n"));
        render_string_id_conversions(&mut out, &id_name);
        if let Some(object_type) = object_type {
            if let Some(space) = object_type.object_space {
                out.push_str(&format!("impl {id_name} {{\n"));
                out.push_str(&format!("    pub const SPACE_ID: u32 = {space};\n"));
                if let Some(type_id) = object_type.type_id {
                    out.push_str(&format!("    pub const TYPE_ID: u32 = {type_id};\n"));
                }
                out.push_str("}\n");
            }
        }
        out.push('\n');
    }

    Ok(out)
}

fn render_string_id_conversions(out: &mut String, id_name: &str) {
    out.push_str(&format!("impl {id_name} {{\n"));
    out.push_str("    pub fn new(value: impl Into<String>) -> Self {\n");
    out.push_str("        Self(value.into())\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");
    out.push_str(&format!("impl From<String> for {id_name} {{\n"));
    out.push_str("    fn from(value: String) -> Self {\n");
    out.push_str("        Self(value)\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");
    out.push_str(&format!("impl From<&str> for {id_name} {{\n"));
    out.push_str("    fn from(value: &str) -> Self {\n");
    out.push_str("        Self(value.to_string())\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");
}

fn detect_schema_no_recursion_cuts(protocol: &Protocol) -> SchemaNoRecursionCuts {
    let graph = build_schema_dependency_graph(protocol);
    let mut cuts = SchemaNoRecursionCuts::default();
    let mut visited = BTreeSet::new();
    let mut visiting = BTreeSet::new();
    let mut path = Vec::new();

    for node in graph.keys() {
        detect_schema_cycles_from(
            node,
            &graph,
            &mut visited,
            &mut visiting,
            &mut path,
            &mut cuts,
        );
    }

    cuts.variants.retain(|variant| variant.owner != "Operation");
    cuts
}

fn build_schema_dependency_graph(protocol: &Protocol) -> BTreeMap<String, Vec<SchemaEdge>> {
    let mut graph = BTreeMap::new();

    for struct_def in &protocol.structs {
        if struct_def.kind == StructKind::Operation || is_operation_ref(protocol, &struct_def.name) {
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
                graph.entry(owner.clone()).or_insert_with(Vec::new).push(SchemaEdge {
                    target,
                    cut: Some(SchemaCut::Variant(variant_key.clone())),
                });
            }
        }
    }

    graph
}

fn add_schema_field_edges(
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
        graph.entry(owner.to_string()).or_insert_with(Vec::new).push(SchemaEdge {
            target,
            cut: Some(SchemaCut::Field(field_key.clone())),
        });
    }
}

fn schema_type_targets(protocol: &Protocol, ty: &TypeRef) -> BTreeSet<String> {
    let mut targets = BTreeSet::new();
    collect_schema_type_targets(protocol, ty, &mut targets);
    targets
}

fn collect_schema_type_targets(protocol: &Protocol, ty: &TypeRef, targets: &mut BTreeSet<String>) {
    match ty {
        TypeRef::Optional { inner }
        | TypeRef::Vector { inner }
        | TypeRef::Set { inner, .. } => collect_schema_type_targets(protocol, inner, targets),
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

fn detect_schema_cycles_from(
    node: &str,
    graph: &BTreeMap<String, Vec<SchemaEdge>>,
    _visited: &mut BTreeSet<String>,
    visiting: &mut BTreeSet<String>,
    path: &mut Vec<DfsEdge>,
    cuts: &mut SchemaNoRecursionCuts,
) {
    if !visiting.insert(node.to_string()) {
        return;
    }

    if let Some(edges) = graph.get(node) {
        for edge in edges {
            let dfs_edge = DfsEdge {
                cut: edge.cut.clone(),
            };
            if visiting.contains(&edge.target) {
                if let Some(cut) = choose_schema_cycle_cut(&edge.target, &dfs_edge, path) {
                    record_schema_cycle_cut(cut, cuts);
                }
                continue;
            }

            path.push(dfs_edge);
            detect_schema_cycles_from(
                &edge.target,
                graph,
                _visited,
                visiting,
                path,
                cuts,
            );
            path.pop();
        }
    }

    visiting.remove(node);
}

fn choose_schema_cycle_cut(
    target: &str,
    back_edge: &DfsEdge,
    path: &[DfsEdge],
) -> Option<SchemaCut> {
    if let Some(SchemaCut::Variant(variant)) = &back_edge.cut {
        if variant.owner != "Operation" {
            return Some(SchemaCut::Variant(variant.clone()));
        }
        if let Some(field) = path.iter().rev().find_map(|edge| match &edge.cut {
            Some(SchemaCut::Field(field)) => Some(field.clone()),
            _ => None,
        }) {
            return Some(SchemaCut::Field(field));
        }
    }

    if let Some(field) = path.iter().rev().find_map(|edge| match &edge.cut {
        Some(SchemaCut::Field(field)) if field.owner == target => Some(field.clone()),
        _ => None,
    }) {
        return Some(SchemaCut::Field(field));
    }

    if let Some(variant) = path.iter().rev().find_map(|edge| match &edge.cut {
        Some(SchemaCut::Variant(variant)) => Some(variant.clone()),
        _ => None,
    }) {
        return Some(SchemaCut::Variant(variant));
    }

    back_edge
        .cut
        .clone()
        .or_else(|| path.iter().rev().find_map(|edge| edge.cut.clone()))
}

fn record_schema_cycle_cut(cut: SchemaCut, cuts: &mut SchemaNoRecursionCuts) {
    match cut {
        SchemaCut::Field(field) => {
            cuts.fields.insert(field);
        }
        SchemaCut::Variant(variant) => {
            cuts.variants.insert(variant);
        }
    }
}

fn render_types(protocol: &Protocol) -> Result<String> {
    let no_recursion_cuts = detect_schema_no_recursion_cuts(protocol);
    let mut out = generated_header(protocol, "raw structs and enums");
    out.push_str("use serde::{Deserialize, Serialize};\n\n");
    // `share_type` (i64-as-number-or-decimal-string) is decoded the same way on every
    // chain, so the helper lives in graphene-core and is re-exported here rather than
    // re-emitted per chain.
    out.push_str(
        "pub(crate) use open_graphene_core::deserialize_i64_from_number_or_decimal_string;\n\n",
    );
    if protocol_uses_fixed_bytes(protocol) {
        // The generic hex-or-byte-array decoder is shared typing; it lives in graphene-core.
        // Only the per-length wrappers below (which lengths a chain uses) stay generated.
        out.push_str(
            "use open_graphene_core::deserialize_fixed_bytes_from_hex_string_or_byte_array;\n\n",
        );
        for len in fixed_byte_lengths(protocol) {
            out.push_str(&format!(
                "pub(crate) fn deserialize_fixed_bytes_{len}_from_hex_string_or_byte_array<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>\n"
            ));
            out.push_str("where\n");
            out.push_str("    D: serde::Deserializer<'de>,\n");
            out.push_str("{\n");
            out.push_str(&format!(
                "    deserialize_fixed_bytes_from_hex_string_or_byte_array(deserializer, {len})\n"
            ));
            out.push_str("}\n\n");
        }
    }

    let mut emitted = BTreeSet::new();
    if protocol_uses_signature(protocol) {
        ensure_unique(&mut emitted, "Signature", "type")?;
        render_signature_type(&mut out, protocol);
    }
    for enum_def in sorted_enums(&protocol.enums) {
        let name = rust_type_name(&enum_def.name);
        ensure_unique(&mut emitted, &name, "type")?;
        render_enum(&mut out, protocol, &enum_def)?;
    }

    for struct_def in sorted_structs(&protocol.structs) {
        if struct_def.kind == StructKind::Operation || is_operation_ref(protocol, &struct_def.name)
        {
            continue;
        }
        let name = rust_type_name(&struct_def.name);
        ensure_unique(&mut emitted, &name, "type")?;
        render_struct(&mut out, protocol, &struct_def, &no_recursion_cuts)?;
    }

    Ok(out)
}

fn render_operations(protocol: &Protocol) -> Result<String> {
    let no_recursion_cuts = detect_schema_no_recursion_cuts(protocol);
    let mut out = generated_header(protocol, "operation structs");
    out.push_str("use serde::{Deserialize, Serialize};\n\n");

    let mut emitted = BTreeSet::new();
    for operation in sorted_operations(&protocol.operations) {
        let name = rust_type_name(&operation.name);
        ensure_unique(&mut emitted, &name, "operation")?;
        out.push_str(&format!(
            "pub const {}: u32 = {};\n\n",
            rust_const_name(&format!("{}_id", operation.name)),
            operation.wire_tag
        ));
        render_operation_struct(&mut out, protocol, &operation, &no_recursion_cuts)?;
    }

    Ok(out)
}

fn render_static_variants(protocol: &Protocol) -> Result<String> {
    let no_recursion_cuts = detect_schema_no_recursion_cuts(protocol);
    let mut out = generated_header(protocol, "static variant enums");
    out.push_str("// Static variants use Graphene JSON wire format: [tag, value].\n\n");

    let mut emitted = BTreeSet::new();
    for variant in sorted_static_variants(&protocol.static_variants) {
        let name = rust_type_name(&variant.name);
        ensure_unique(&mut emitted, &name, "static variant")?;
        render_static_variant(&mut out, protocol, &variant, &no_recursion_cuts)?;
    }

    Ok(out)
}

fn render_signature_type(out: &mut String, protocol: &Protocol) {
    out.push_str("/// Graphene compact recoverable ECDSA signature bytes.\n");
    out.push_str("/// Wire layout: one compact header byte followed by 32-byte r and 32-byte s.\n");
    // The node encodes the signature as a hex string (e.g. in a `signed_block`), so the bytes carry
    // the same hex serde as any other `Bytes` field. Without this the derived `Vec<u8>` (de)serialize
    // would expect a JSON number array and fail to parse the node's hex.
    out.push_str(
        "#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]\n",
    );
    out.push_str(&format!(
        "#[schema(as = {}, value_type = String)]\n",
        openapi_schema_name(protocol, "Signature")
    ));
    out.push_str("pub struct Signature(\n");
    out.push_str("    #[serde(serialize_with = \"open_graphene_core::serialize_bytes_as_hex\", deserialize_with = \"open_graphene_core::deserialize_bytes_from_hex_string_or_byte_array\")]\n");
    out.push_str("    pub Vec<u8>,\n");
    out.push_str(");\n\n");
}

fn render_enum(out: &mut String, protocol: &Protocol, enum_def: &EnumDef) -> Result<()> {
    let enum_name = rust_type_name(&enum_def.name);
    out.push_str(&format!(
        "/// Raw enum `{}`. Numeric wire serde is not implemented yet.\n",
        enum_def.name
    ));
    out.push_str(
        "#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]\n",
    );
    out.push_str(&format!(
        "#[schema(as = {})]\n",
        openapi_schema_name(protocol, &enum_name)
    ));
    out.push_str(&format!("pub enum {enum_name} {{\n"));

    let mut variants = enum_def.values.clone();
    variants.sort_by_key(|value| value.value);
    let mut emitted = BTreeSet::new();
    for value in variants {
        let variant_name = rust_variant_name(&value.name);
        ensure_unique(&mut emitted, &variant_name, "enum variant")?;
        out.push_str(&serde_rename_attr("    ", &value.name, &variant_name));
        out.push_str(&format!("    {variant_name},\n"));
    }

    out.push_str("}\n\n");
    Ok(())
}

fn render_struct(
    out: &mut String,
    protocol: &Protocol,
    struct_def: &StructDef,
    no_recursion_cuts: &SchemaNoRecursionCuts,
) -> Result<()> {
    let struct_name = rust_type_name(&struct_def.name);
    out.push_str(&format!("/// Raw protocol struct `{}`.\n", struct_def.name));
    out.push_str("#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]\n");
    out.push_str(&format!(
        "#[schema(as = {})]\n",
        openapi_schema_name(protocol, &struct_name)
    ));
    out.push_str(&format!("pub struct {struct_name} {{\n"));
    render_fields(out, protocol, &struct_name, &struct_def.fields, no_recursion_cuts)?;
    out.push_str("}\n\n");
    render_asset_constructor(out, protocol, struct_def)?;
    render_price_constructor(out, protocol, struct_def)?;
    Ok(())
}

fn render_asset_constructor(
    out: &mut String,
    protocol: &Protocol,
    struct_def: &StructDef,
) -> Result<()> {
    if struct_def.name != "asset" {
        return Ok(());
    }

    let mut fields = struct_def.fields.clone();
    fields.sort_by_key(|field| field.index);
    let amount = fields.iter().find(|field| field.name == "amount");
    let asset_id = fields.iter().find(|field| field.name == "asset_id");

    if let (Some(amount), Some(asset_id)) = (amount, asset_id) {
        let amount_ty = render_type_ref(protocol, &amount.ty)?;
        let asset_id_ty = render_type_ref(protocol, &asset_id.ty)?;
        out.push_str("impl Asset {\n");
        out.push_str(&format!(
            "    pub fn new(amount: {amount_ty}, asset_id: {asset_id_ty}) -> Self {{\n"
        ));
        out.push_str("        Self { amount, asset_id }\n");
        out.push_str("    }\n");
        out.push_str("}\n\n");
    }

    Ok(())
}

fn render_price_constructor(
    out: &mut String,
    protocol: &Protocol,
    struct_def: &StructDef,
) -> Result<()> {
    if struct_def.name != "price" {
        return Ok(());
    }

    let mut fields = struct_def.fields.clone();
    fields.sort_by_key(|field| field.index);
    let base = fields.iter().find(|field| field.name == "base");
    let quote = fields.iter().find(|field| field.name == "quote");

    if let (Some(base), Some(quote)) = (base, quote) {
        let base_ty = render_type_ref(protocol, &base.ty)?;
        let quote_ty = render_type_ref(protocol, &quote.ty)?;
        out.push_str("impl Price {\n");
        out.push_str(&format!(
            "    pub fn new(base: {base_ty}, quote: {quote_ty}) -> Self {{\n"
        ));
        out.push_str("        Self { base, quote }\n");
        out.push_str("    }\n");
        out.push_str("}\n\n");
    }

    Ok(())
}

fn render_operation_struct(
    out: &mut String,
    protocol: &Protocol,
    operation: &OperationDef,
    no_recursion_cuts: &SchemaNoRecursionCuts,
) -> Result<()> {
    let struct_name = rust_type_name(&operation.name);
    out.push_str(&format!(
        "/// Protocol operation `{}` with wire tag {}.\n",
        operation.name, operation.wire_tag
    ));
    out.push_str("#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]\n");
    out.push_str(&format!(
        "#[schema(as = {})]\n",
        openapi_schema_name(protocol, &struct_name)
    ));
    out.push_str(&format!("pub struct {struct_name} {{\n"));
    render_fields(out, protocol, &struct_name, &operation.fields, no_recursion_cuts)?;
    out.push_str("}\n\n");
    Ok(())
}

fn render_fields(
    out: &mut String,
    protocol: &Protocol,
    owner: &str,
    fields: &[FieldDef],
    no_recursion_cuts: &SchemaNoRecursionCuts,
) -> Result<()> {
    let mut fields = fields.to_vec();
    fields.sort_by_key(|field| field.index);
    let mut emitted = BTreeSet::new();

    for field in fields {
        let field_name = rust_field_name(&field.name);
        ensure_unique(&mut emitted, &field_name, "field")?;
        let ty = render_type_ref(protocol, &field.ty)?;
        out.push_str(&serde_rename_attr("    ", &field.name, &field_name));
        if matches!(field.ty, TypeRef::Int64 { .. }) {
            out.push_str(
                "    #[serde(deserialize_with = \"crate::generated::types::deserialize_i64_from_number_or_decimal_string\")]\n",
            );
        }
        // Byte fields are hex strings on the Graphene wire: decode hex-or-array, encode as hex.
        if let TypeRef::FixedBytes { bytes } = field.ty {
            out.push_str(&format!(
                "    #[serde(serialize_with = \"open_graphene_core::serialize_bytes_as_hex\", deserialize_with = \"crate::generated::types::deserialize_fixed_bytes_{bytes}_from_hex_string_or_byte_array\")]\n"
            ));
            out.push_str("    #[schema(value_type = String)]\n");
        }
        if matches!(field.ty, TypeRef::Bytes) {
            out.push_str(
                "    #[serde(serialize_with = \"open_graphene_core::serialize_bytes_as_hex\", deserialize_with = \"open_graphene_core::deserialize_bytes_from_hex_string_or_byte_array\")]\n",
            );
            out.push_str("    #[schema(value_type = String)]\n");
        }
        if no_recursion_cuts.fields.contains(&SchemaFieldKey {
            owner: owner.to_string(),
            field: field_name.clone(),
        }) {
            out.push_str("    #[schema(no_recursion)]\n");
        }
        out.push_str(&format!("    pub {field_name}: {ty},\n"));
    }

    Ok(())
}

fn render_fc(protocol: &Protocol) -> Result<String> {
    let mut out = generated_header(protocol, "minimal FC serialization for transfer path");
    out.push_str("pub use open_graphene_fc::{decode_chain_id_hex, decode_public_key, is_graphene_canonical_compact_signature, parse_protocol_object_id, recover_public_key_from_compact_signature, sha256_bytes, sign_digest_compact_with_wif, verify_compact_signature_public_key, write_bytes, write_fixed_bytes, write_protocol_object_id, write_public_key, write_time_point_sec, write_varint, write_vote_id, FcSerialize, FcSerializeError, Result};\n\n");

    render_fc_id_impls(&mut out, protocol)?;
    render_fc_signature_impl(&mut out, protocol);
    render_fc_transfer_path_impls(&mut out, protocol)?;
    Ok(out)
}

fn render_fc_signature_impl(out: &mut String, protocol: &Protocol) {
    if !protocol_uses_signature(protocol) {
        return;
    }

    out.push_str("impl FcSerialize for crate::generated::types::Signature {\n");
    out.push_str("    fn fc_serialize(&self, out: &mut Vec<u8>) -> Result<()> {\n");
    out.push_str("        write_fixed_bytes(&self.0, 65, \"signature\", out)\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");
}

fn render_fc_id_impls(out: &mut String, protocol: &Protocol) -> Result<()> {
    let mut protocol_object_ids = collect_protocol_object_id_names(protocol);
    protocol_object_ids.extend(
        protocol
            .object_types
            .iter()
            .map(|object_type| object_type.object_type.clone()),
    );

    for object_type_name in protocol_object_ids {
        let id_name = object_id_type_name(&object_type_name);
        let object_type = protocol
            .object_types
            .iter()
            .find(|object_type| object_type.object_type == object_type_name);
        let expected_space = object_type
            .and_then(|object_type| object_type.object_space)
            .map_or("None".to_string(), |value| format!("Some({value})"));
        let expected_type = object_type
            .and_then(|object_type| object_type.type_id)
            .map_or("None".to_string(), |value| format!("Some({value})"));

        out.push_str(&format!(
            "impl FcSerialize for crate::generated::ids::{id_name} {{\n"
        ));
        out.push_str("    fn fc_serialize(&self, out: &mut Vec<u8>) -> Result<()> {\n");
        if id_name == "VoteId" {
            out.push_str("        write_vote_id(&self.0, out)\n");
        } else {
            out.push_str(&format!(
                "        write_protocol_object_id(&self.0, {expected_space}, {expected_type}, out)\n"
            ));
        }
        out.push_str("    }\n");
        out.push_str("}\n\n");
    }

    out.push_str("impl FcSerialize for crate::generated::ids::ObjectId {\n");
    out.push_str("    fn fc_serialize(&self, out: &mut Vec<u8>) -> Result<()> {\n");
    out.push_str("        write_protocol_object_id(&self.0, None, None, out)\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");
    Ok(())
}

fn render_fc_transfer_path_impls(out: &mut String, protocol: &Protocol) -> Result<()> {
    let supported_structs = render_fc_struct_impls(out, protocol)?;

    if protocol
        .static_variants
        .iter()
        .any(|variant| variant.name == "future_extensions")
    {
        out.push_str(
            "fn future_extensions_tag(value: &crate::generated::static_variants::FutureExtensions) -> u64 {\n",
        );
        out.push_str("    match value {\n");
        out.push_str(
            "        crate::generated::static_variants::FutureExtensions::VoidT(_) => 0u64,\n",
        );
        out.push_str("    }\n");
        out.push_str("}\n\n");

        out.push_str(
            "impl FcSerialize for crate::generated::static_variants::FutureExtensions {\n",
        );
        out.push_str("    fn fc_serialize(&self, out: &mut Vec<u8>) -> Result<()> {\n");
        out.push_str("        match self {\n");
        out.push_str("            Self::VoidT(value) => {\n");
        out.push_str("                write_varint(0, out);\n");
        out.push_str("                value.as_ref().fc_serialize(out)\n");
        out.push_str("            }\n");
        out.push_str("        }\n");
        out.push_str("    }\n");
        out.push_str("}\n\n");
    }

    if protocol
        .static_variants
        .iter()
        .any(|variant| variant.name == "special_authority")
    {
        out.push_str(
            "impl FcSerialize for crate::generated::static_variants::SpecialAuthority {\n",
        );
        out.push_str("    fn fc_serialize(&self, out: &mut Vec<u8>) -> Result<()> {\n");
        out.push_str("        match self {\n");
        out.push_str("            Self::NoSpecialAuthority(value) => {\n");
        out.push_str("                write_varint(0, out);\n");
        out.push_str("                value.as_ref().fc_serialize(out)\n");
        out.push_str("            }\n");
        out.push_str("            Self::TopHoldersSpecialAuthority(value) => {\n");
        out.push_str("                write_varint(1, out);\n");
        out.push_str("                value.as_ref().fc_serialize(out)\n");
        out.push_str("            }\n");
        out.push_str("        }\n");
        out.push_str("    }\n");
        out.push_str("}\n\n");
    }

    render_fc_htlc_hash_impl(out, protocol)?;
    render_fc_predicate_impl(out, protocol)?;
    render_fc_vesting_policy_initializer_impl(out, protocol)?;
    render_fc_worker_initializer_impl(out, protocol)?;
    render_fc_limit_order_auto_action_impl(out, protocol)?;
    render_fc_fee_parameters_impl(out, protocol)?;
    render_fc_argument_type_impl(out, protocol)?;

    let supported_operations = render_fc_operation_impls(out, protocol, &supported_structs)?;

    let operation_variant = protocol
        .static_variants
        .iter()
        .find(|variant| variant.name == "operation");
    if let Some(operation_variant) = operation_variant {
        out.push_str("impl FcSerialize for crate::generated::static_variants::Operation {\n");
        out.push_str("    fn fc_serialize(&self, out: &mut Vec<u8>) -> Result<()> {\n");
        out.push_str("        match self {\n");
        let mut arms = operation_variant.variants.clone();
        arms.sort_by_key(|arm| arm.tag);
        for arm in arms {
            let variant_name = rust_variant_name(&arm.name);
            if supported_operations.contains(&arm.name) {
                out.push_str(&format!(
                    "            Self::{variant_name}(value) => {{ write_varint({}u64, out); value.as_ref().fc_serialize(out) }}\n",
                    arm.tag
                ));
            } else {
                out.push_str(&format!(
                    "            Self::{variant_name}(_) => Err(FcSerializeError::UnsupportedVariant {{ variant: {} }}),\n",
                    rust_string_literal(&variant_name)
                ));
            }
        }
        out.push_str("        }\n");
        out.push_str("    }\n");
        out.push_str("}\n\n");
    }

    render_fc_transaction_helpers(out, protocol);

    Ok(())
}

fn render_fc_transaction_helpers(out: &mut String, protocol: &Protocol) {
    if protocol.chain.chain_id.is_none()
        || !protocol
            .structs
            .iter()
            .any(|struct_def| struct_def.name == "transaction")
    {
        return;
    }
    let has_signed_transaction = protocol
        .structs
        .iter()
        .any(|struct_def| struct_def.name == "signed_transaction");

    out.push_str("impl crate::generated::types::Transaction {\n");
    out.push_str("    pub fn signature_preimage_bytes(&self) -> Result<Vec<u8>> {\n");
    out.push_str("        let mut out = Vec::new();\n");
    out.push_str("        out.extend_from_slice(&decode_chain_id_hex(crate::generated::ids::CHAIN_ID_HEX)?);\n");
    out.push_str("        self.fc_serialize(&mut out)?;\n");
    out.push_str("        Ok(out)\n");
    out.push_str("    }\n\n");
    out.push_str("    pub fn signature_digest_bytes(&self) -> Result<[u8; 32]> {\n");
    out.push_str("        Ok(sha256_bytes(&self.signature_preimage_bytes()?))\n");
    out.push_str("    }\n\n");
    out.push_str("    pub fn sign_with_wif(&self, wif: &str) -> Result<crate::generated::types::Signature> {\n");
    out.push_str("        Ok(crate::generated::types::Signature(sign_digest_compact_with_wif(self.signature_digest_bytes()?, wif)?.to_vec()))\n");
    out.push_str("    }\n");
    if has_signed_transaction {
        out.push_str("\n");
        out.push_str("    pub fn signed_with_wif(&self, wif: &str) -> Result<crate::generated::types::SignedTransaction> {\n");
        out.push_str("        Ok(crate::generated::types::SignedTransaction {\n");
        out.push_str("            ref_block_num: self.ref_block_num,\n");
        out.push_str("            ref_block_prefix: self.ref_block_prefix,\n");
        out.push_str("            expiration: self.expiration.clone(),\n");
        out.push_str("            operations: self.operations.clone(),\n");
        out.push_str("            extensions: self.extensions.clone(),\n");
        out.push_str("            signatures: vec![self.sign_with_wif(wif)?],\n");
        out.push_str("        })\n");
        out.push_str("    }\n");
    }
    out.push_str("}\n\n");
}

fn render_fc_struct_impls(out: &mut String, protocol: &Protocol) -> Result<BTreeSet<String>> {
    let supported_structs = fc_supported_struct_names(protocol);
    let extension_structs = extension_struct_names(protocol);

    for struct_def in sorted_structs(&protocol.structs) {
        if !supported_structs.contains(&struct_def.name) {
            continue;
        }

        // A struct standing in for a graphene `extension<T>` serializes count-prefixed (a varint
        // of how many members are set), not as a plain run of optionals. Empty is a single `0`.
        if extension_structs.contains(&struct_def.name) {
            render_fc_extension_struct_impl(out, &struct_def);
            continue;
        }

        let struct_name = rust_type_name(&struct_def.name);
        out.push_str(&format!(
            "impl FcSerialize for crate::generated::types::{struct_name} {{\n"
        ));
        out.push_str("    fn fc_serialize(&self, out: &mut Vec<u8>) -> Result<()> {\n");
        let mut fields = struct_def.fields.clone();
        fields.sort_by_key(|field| field.index);
        if fields.is_empty() {
            out.push_str("        let _ = out;\n");
        }
        for field in fields {
            out.push_str(&render_fc_field_serialize_line(&field)?);
        }
        out.push_str("        Ok(())\n");
        out.push_str("    }\n");
        out.push_str("}\n\n");
    }

    Ok(supported_structs)
}

/// Struct names that stand in for a graphene `extension<T>`: anything referenced as the type of an
/// `extensions` field (the `flat_set<future_extension>` variant is an array, not a struct ref, so
/// it never lands here).
fn extension_struct_names(protocol: &Protocol) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut collect = |fields: &[FieldDef]| {
        for field in fields {
            if field.name == "extensions"
                && let TypeRef::Ref { name } = &field.ty
            {
                names.insert(name.clone());
            }
        }
    };
    for struct_def in &protocol.structs {
        collect(&struct_def.fields);
    }
    for operation in &protocol.operations {
        collect(&operation.fields);
    }
    names
}

/// FC encoding for a `extension<T>` struct: a varint count of set members followed by each set
/// member. An empty set (the common case) is a single `0`. Setting members is rejected for now,
/// the same stance the hand-written impls took, until per-member encoding is wired up.
fn render_fc_extension_struct_impl(out: &mut String, struct_def: &StructDef) {
    let struct_name = rust_type_name(&struct_def.name);
    out.push_str(&format!(
        "impl FcSerialize for crate::generated::types::{struct_name} {{\n"
    ));
    out.push_str("    fn fc_serialize(&self, out: &mut Vec<u8>) -> Result<()> {\n");

    let mut fields = struct_def.fields.clone();
    fields.sort_by_key(|field| field.index);

    if fields.is_empty() {
        out.push_str("        write_varint(0u64, out);\n");
        out.push_str("        Ok(())\n");
    } else {
        let empty_check = fields
            .iter()
            .map(|field| format!("self.{}.is_none()", rust_field_name(&field.name)))
            .collect::<Vec<_>>()
            .join(" && ");
        out.push_str(&format!("        if {empty_check} {{\n"));
        out.push_str("            write_varint(0u64, out);\n");
        out.push_str("            return Ok(());\n");
        out.push_str("        }\n");
        out.push_str("        Err(FcSerializeError::UnsupportedValue {\n");
        out.push_str(&format!(
            "            type_name: \"{}\",\n",
            struct_def.name
        ));
        out.push_str("            reason: \"non-empty graphene extension set is not supported by FC serialization yet\",\n");
        out.push_str("        })\n");
    }

    out.push_str("    }\n");
    out.push_str("}\n\n");
}

fn render_fc_htlc_hash_impl(out: &mut String, protocol: &Protocol) -> Result<()> {
    let Some(variant) = protocol
        .static_variants
        .iter()
        .find(|variant| variant.name == "htlc_hash")
    else {
        return Ok(());
    };

    out.push_str("impl FcSerialize for crate::generated::static_variants::HtlcHash {\n");
    out.push_str("    fn fc_serialize(&self, out: &mut Vec<u8>) -> Result<()> {\n");
    out.push_str("        match self {\n");

    let mut arms = variant.variants.clone();
    arms.sort_by_key(|arm| arm.tag);
    for arm in arms {
        let TypeRef::FixedBytes { bytes } = arm.ty else {
            return Err(GenBindingsRsError::Render {
                message: format!(
                    "unsupported htlc_hash variant `{}` payload type for FC rendering",
                    arm.name
                ),
            });
        };
        let variant_name = rust_variant_name(&arm.name);
        out.push_str(&format!(
            "            Self::{variant_name}(value) => {{\n                write_varint({}u64, out);\n                write_fixed_bytes(value.as_ref(), {bytes}, {}, out)\n            }}\n",
            arm.tag,
            rust_string_literal(&format!("htlc_hash::{}", arm.name))
        ));
    }

    out.push_str("        }\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");
    Ok(())
}

fn render_fc_predicate_impl(out: &mut String, protocol: &Protocol) -> Result<()> {
    let Some(variant) = protocol
        .static_variants
        .iter()
        .find(|variant| variant.name == "predicate")
    else {
        return Ok(());
    };

    out.push_str("impl FcSerialize for crate::generated::static_variants::Predicate {\n");
    out.push_str("    fn fc_serialize(&self, out: &mut Vec<u8>) -> Result<()> {\n");
    out.push_str("        match self {\n");

    let mut arms = variant.variants.clone();
    arms.sort_by_key(|arm| arm.tag);
    for arm in arms {
        if !matches!(arm.ty, TypeRef::Ref { .. }) {
            return Err(GenBindingsRsError::Render {
                message: format!(
                    "unsupported predicate variant `{}` payload type for FC rendering",
                    arm.name
                ),
            });
        }
        let variant_name = rust_variant_name(&arm.name);
        out.push_str(&format!(
            "            Self::{variant_name}(value) => {{\n                write_varint({}u64, out);\n                value.as_ref().fc_serialize(out)\n            }}\n",
            arm.tag
        ));
    }

    out.push_str("        }\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");
    Ok(())
}

fn render_fc_vesting_policy_initializer_impl(out: &mut String, protocol: &Protocol) -> Result<()> {
    let Some(variant) = protocol
        .static_variants
        .iter()
        .find(|variant| variant.name == "vesting_policy_initializer")
    else {
        return Ok(());
    };

    out.push_str(
        "impl FcSerialize for crate::generated::static_variants::VestingPolicyInitializer {\n",
    );
    out.push_str("    fn fc_serialize(&self, out: &mut Vec<u8>) -> Result<()> {\n");
    out.push_str("        match self {\n");

    let mut arms = variant.variants.clone();
    arms.sort_by_key(|arm| arm.tag);
    for arm in arms {
        if !matches!(arm.ty, TypeRef::Ref { .. }) {
            return Err(GenBindingsRsError::Render {
                message: format!(
                    "unsupported vesting_policy_initializer variant `{}` payload type for FC rendering",
                    arm.name
                ),
            });
        }
        let variant_name = rust_variant_name(&arm.name);
        out.push_str(&format!(
            "            Self::{variant_name}(value) => {{\n                write_varint({}u64, out);\n                value.as_ref().fc_serialize(out)\n            }}\n",
            arm.tag
        ));
    }

    out.push_str("        }\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");
    Ok(())
}

fn render_fc_worker_initializer_impl(out: &mut String, protocol: &Protocol) -> Result<()> {
    let Some(variant) = protocol
        .static_variants
        .iter()
        .find(|variant| variant.name == "worker_initializer")
    else {
        return Ok(());
    };

    out.push_str("impl FcSerialize for crate::generated::static_variants::WorkerInitializer {\n");
    out.push_str("    fn fc_serialize(&self, out: &mut Vec<u8>) -> Result<()> {\n");
    out.push_str("        match self {\n");

    let mut arms = variant.variants.clone();
    arms.sort_by_key(|arm| arm.tag);
    for arm in arms {
        if !matches!(arm.ty, TypeRef::Ref { .. }) {
            return Err(GenBindingsRsError::Render {
                message: format!(
                    "unsupported worker_initializer variant `{}` payload type for FC rendering",
                    arm.name
                ),
            });
        }
        let variant_name = rust_variant_name(&arm.name);
        out.push_str(&format!(
            "            Self::{variant_name}(value) => {{\n                write_varint({}u64, out);\n                value.as_ref().fc_serialize(out)\n            }}\n",
            arm.tag
        ));
    }

    out.push_str("        }\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");
    Ok(())
}

fn render_fc_limit_order_auto_action_impl(out: &mut String, protocol: &Protocol) -> Result<()> {
    let Some(variant) = protocol
        .static_variants
        .iter()
        .find(|variant| variant.name == "limit_order_auto_action")
    else {
        return Ok(());
    };

    out.push_str(
        "impl FcSerialize for crate::generated::static_variants::LimitOrderAutoAction {\n",
    );
    out.push_str("    fn fc_serialize(&self, out: &mut Vec<u8>) -> Result<()> {\n");
    out.push_str("        match self {\n");

    let mut arms = variant.variants.clone();
    arms.sort_by_key(|arm| arm.tag);
    for arm in arms {
        if !matches!(arm.ty, TypeRef::Ref { .. }) {
            return Err(GenBindingsRsError::Render {
                message: format!(
                    "unsupported limit_order_auto_action variant `{}` payload type for FC rendering",
                    arm.name
                ),
            });
        }
        let variant_name = rust_variant_name(&arm.name);
        out.push_str(&format!(
            "            Self::{variant_name}(value) => {{\n                write_varint({}u64, out);\n                value.as_ref().fc_serialize(out)\n            }}\n",
            arm.tag
        ));
    }

    out.push_str("        }\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");
    Ok(())
}

fn render_fc_fee_parameters_impl(out: &mut String, protocol: &Protocol) -> Result<()> {
    let Some(variant) = protocol
        .static_variants
        .iter()
        .find(|variant| variant.name == "fee_parameters")
    else {
        return Ok(());
    };

    out.push_str("fn fee_parameters_tag(value: &crate::generated::static_variants::FeeParameters) -> u64 {\n");
    out.push_str("    match value {\n");
    let mut arms = variant.variants.clone();
    arms.sort_by_key(|arm| arm.tag);
    for arm in &arms {
        let variant_name = rust_variant_name(&arm.name);
        out.push_str(&format!(
            "        crate::generated::static_variants::FeeParameters::{variant_name}(_) => {}u64,\n",
            arm.tag
        ));
    }
    out.push_str("    }\n");
    out.push_str("}\n\n");

    out.push_str("impl FcSerialize for crate::generated::static_variants::FeeParameters {\n");
    out.push_str("    fn fc_serialize(&self, out: &mut Vec<u8>) -> Result<()> {\n");
    out.push_str("        match self {\n");

    for arm in arms {
        if !matches!(arm.ty, TypeRef::Ref { .. }) {
            return Err(GenBindingsRsError::Render {
                message: format!(
                    "unsupported fee_parameters variant `{}` payload type for FC rendering",
                    arm.name
                ),
            });
        }
        let variant_name = rust_variant_name(&arm.name);
        out.push_str(&format!(
            "            Self::{variant_name}(value) => {{\n                write_varint({}u64, out);\n                value.as_ref().fc_serialize(out)\n            }}\n",
            arm.tag
        ));
    }

    out.push_str("        }\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");
    Ok(())
}

fn render_fc_argument_type_impl(out: &mut String, protocol: &Protocol) -> Result<()> {
    let Some(variant) = protocol
        .static_variants
        .iter()
        .find(|variant| variant.name == "argument_type")
    else {
        return Ok(());
    };

    out.push_str("impl FcSerialize for crate::generated::static_variants::ArgumentType {\n");
    out.push_str("    fn fc_serialize(&self, out: &mut Vec<u8>) -> Result<()> {\n");
    out.push_str("        match self {\n");

    let mut arms = variant.variants.clone();
    arms.sort_by_key(|arm| arm.tag);
    let supported_structs = fc_supported_struct_names(protocol);
    for arm in arms {
        if !is_fc_supported_type(protocol, &arm.ty, &supported_structs) {
            return Err(GenBindingsRsError::Render {
                message: format!(
                    "unsupported argument_type variant `{}` payload type for FC rendering",
                    arm.name
                ),
            });
        }
        let variant_name = rust_variant_name(&arm.name);
        let payload_lines =
            render_fc_value_serialize_lines("(**value)", &arm.ty, "                ")?;
        out.push_str(&format!(
            "            Self::{variant_name}(value) => {{\n                write_varint({}u64, out);\n{payload_lines}                Ok(())\n            }}\n",
            arm.tag
        ));
    }

    out.push_str("        }\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");
    Ok(())
}

fn fc_supported_struct_names(protocol: &Protocol) -> BTreeSet<String> {
    let mut supported = BTreeSet::new();
    let mut changed = true;

    while changed {
        changed = false;
        for struct_def in sorted_structs(&protocol.structs) {
            if supported.contains(&struct_def.name)
                || struct_def.kind == StructKind::Operation
                || is_operation_ref(protocol, &struct_def.name)
            {
                continue;
            }

            if struct_def
                .fields
                .iter()
                .all(|field| is_fc_supported_type(protocol, &field.ty, &supported))
            {
                supported.insert(struct_def.name);
                changed = true;
            }
        }
    }

    supported
}

fn render_fc_operation_impls(
    out: &mut String,
    protocol: &Protocol,
    supported_structs: &BTreeSet<String>,
) -> Result<BTreeSet<String>> {
    let mut supported_operations = BTreeSet::new();

    for operation in sorted_operations(&protocol.operations) {
        if operation.name == "transfer_operation" {
            render_fc_transfer_operation_impl(out);
            supported_operations.insert(operation.name);
            continue;
        }

        if !is_fc_supported_operation(protocol, &operation, supported_structs) {
            continue;
        }

        let operation_name = rust_type_name(&operation.name);
        out.push_str(&format!(
            "impl FcSerialize for crate::generated::operations::{operation_name} {{\n"
        ));
        out.push_str("    fn fc_serialize(&self, out: &mut Vec<u8>) -> Result<()> {\n");
        let mut fields = operation.fields.clone();
        fields.sort_by_key(|field| field.index);
        if fields.is_empty() {
            out.push_str("        let _ = out;\n");
        }
        for field in fields {
            out.push_str(&render_fc_field_serialize_line(&field)?);
        }
        out.push_str("        Ok(())\n");
        out.push_str("    }\n");
        out.push_str("}\n\n");
        supported_operations.insert(operation.name);
    }

    Ok(supported_operations)
}

fn render_fc_field_serialize_line(field: &FieldDef) -> Result<String> {
    let field_name = rust_field_name(&field.name);
    render_fc_value_serialize_lines(&format!("self.{field_name}"), &field.ty, "        ")
}

fn render_fc_value_serialize_lines(value_expr: &str, ty: &TypeRef, indent: &str) -> Result<String> {
    match ty {
        TypeRef::PublicKey { .. } => {
            let prefix = render_public_key_prefix_expr(ty)?;
            Ok(format!(
                "{indent}write_public_key(&{value_expr}, {prefix}, out)?;\n"
            ))
        }
        TypeRef::TimePointSec => Ok(format!(
            "{indent}write_time_point_sec(&{value_expr}, out)?;\n"
        )),
        ty if is_vote_id_type(ty) => Ok(format!(
            "{indent}write_vote_id({}, out)?;\n",
            render_vote_id_arg(value_expr, ty)?
        )),
        TypeRef::Bytes => Ok(format!("{indent}write_bytes(&{value_expr}, out)?;\n")),
        TypeRef::FixedBytes { bytes } => Ok(format!(
            "{indent}write_fixed_bytes(&{value_expr}, {bytes}, {}, out)?;\n",
            rust_string_literal(&format!("fixed_bytes_{bytes}"))
        )),
        TypeRef::Optional { inner } if matches!(inner.as_ref(), TypeRef::PublicKey { .. }) => {
            let prefix = render_public_key_prefix_expr(inner)?;
            Ok(format!(
                "{indent}match &{value_expr} {{\n\
                 {indent}    Some(value) => {{\n\
                 {indent}        out.push(1);\n\
                 {indent}        write_public_key(value, {prefix}, out)?;\n\
                 {indent}    }}\n\
                 {indent}    None => out.push(0),\n\
                 {indent}}}\n"
            ))
        }
        TypeRef::Optional { inner } if matches!(inner.as_ref(), TypeRef::TimePointSec) => {
            Ok(format!(
                "{indent}match &{value_expr} {{\n\
             {indent}    Some(value) => {{\n\
             {indent}        out.push(1);\n\
             {indent}        write_time_point_sec(value, out)?;\n\
             {indent}    }}\n\
             {indent}    None => out.push(0),\n\
             {indent}}}\n"
            ))
        }
        TypeRef::Optional { inner } if is_vote_id_type(inner) => Ok(format!(
            "{indent}match &{value_expr} {{\n\
             {indent}    Some(value) => {{\n\
             {indent}        out.push(1);\n\
             {indent}        write_vote_id({}, out)?;\n\
             {indent}    }}\n\
             {indent}    None => out.push(0),\n\
             {indent}}}\n",
            render_vote_id_arg("value", inner)?
        )),
        TypeRef::Optional { inner } if matches!(inner.as_ref(), TypeRef::FlatMap { key, value, .. } if is_fc_supported_flat_map(key, value)) =>
        {
            let TypeRef::FlatMap { key, value, .. } = inner.as_ref() else {
                unreachable!("guard checked flat_map inner")
            };
            let inner_lines = render_fc_flat_map_serialize_lines("(*value)", key, value, indent)?;
            Ok(format!(
                "{indent}match &{value_expr} {{\n\
                 {indent}    Some(value) => {{\n\
                 {indent}        out.push(1);\n{inner_lines}\
                 {indent}    }}\n\
                 {indent}    None => out.push(0),\n\
                 {indent}}}\n"
            ))
        }
        TypeRef::Vector { inner } if matches!(inner.as_ref(), TypeRef::PublicKey { .. }) => {
            let prefix = render_public_key_prefix_expr(inner)?;
            Ok(format!(
                "{indent}write_varint({value_expr}.len() as u64, out);\n\
                 {indent}for value in &{value_expr} {{\n\
                 {indent}    write_public_key(value, {prefix}, out)?;\n\
                 {indent}}}\n"
            ))
        }
        TypeRef::Vector { inner } if matches!(inner.as_ref(), TypeRef::TimePointSec) => {
            Ok(format!(
                "{indent}write_varint({value_expr}.len() as u64, out);\n\
             {indent}for value in &{value_expr} {{\n\
             {indent}    write_time_point_sec(value, out)?;\n\
             {indent}}}\n"
            ))
        }
        TypeRef::Vector { inner } | TypeRef::Set { inner, .. } if is_vote_id_type(inner) => {
            Ok(format!(
                "{indent}write_varint({value_expr}.len() as u64, out);\n\
             {indent}for value in &{value_expr} {{\n\
             {indent}    write_vote_id({}, out)?;\n\
             {indent}}}\n",
                render_vote_id_arg("value", inner)?
            ))
        }
        TypeRef::Pair { first, second } => {
            let first_lines =
                render_fc_value_serialize_lines(&format!("{value_expr}.0"), first, indent)?;
            let second_lines =
                render_fc_value_serialize_lines(&format!("{value_expr}.1"), second, indent)?;
            Ok(format!("{first_lines}{second_lines}"))
        }
        TypeRef::Set { inner, .. }
            if is_fee_parameters_type(inner) || is_future_extensions_type(inner) =>
        {
            render_fc_static_variant_set_serialize_lines(value_expr, inner, indent)
        }
        TypeRef::Set { inner, .. } if is_fc_supported_set(inner) => {
            render_fc_set_serialize_lines(value_expr, inner, indent)
        }
        TypeRef::FlatMap { key, value, .. } if is_fc_supported_flat_map(key, value) => {
            render_fc_flat_map_serialize_lines(value_expr, key, value, indent)
        }
        _ => Ok(format!("{indent}{value_expr}.fc_serialize(out)?;\n")),
    }
}

fn is_vote_id_type(ty: &TypeRef) -> bool {
    match ty {
        TypeRef::VoteId => true,
        TypeRef::ProtocolObjectId { object_type } => object_type == "vote",
        _ => false,
    }
}

fn render_fc_set_serialize_lines(
    value_expr: &str,
    inner: &TypeRef,
    indent: &str,
) -> Result<String> {
    match inner {
        TypeRef::Bool => Ok(format!(
            "{indent}write_varint({value_expr}.len() as u64, out);\n\
             {indent}let mut previous_key: Option<bool> = None;\n\
             {indent}for value in &{value_expr} {{\n\
             {indent}    if previous_key.is_some_and(|previous| previous >= *value) {{\n\
             {indent}        return Err(FcSerializeError::UnsupportedValue {{ type_name: \"Set\", reason: \"set values must be sorted and unique\" }});\n\
             {indent}    }}\n\
             {indent}    previous_key = Some(*value);\n\
             {indent}    value.fc_serialize(out)?;\n\
             {indent}}}\n"
        )),
        TypeRef::Uint16 => render_fc_ordered_copy_set_serialize_lines(value_expr, "u16", indent),
        TypeRef::Uint32 => render_fc_ordered_copy_set_serialize_lines(value_expr, "u32", indent),
        TypeRef::Int32 { .. } => {
            render_fc_ordered_copy_set_serialize_lines(value_expr, "i32", indent)
        }
        TypeRef::Int64 { json: None, .. } => {
            render_fc_ordered_copy_set_serialize_lines(value_expr, "i64", indent)
        }
        TypeRef::Uint64 { json: None, .. } => {
            render_fc_ordered_copy_set_serialize_lines(value_expr, "u64", indent)
        }
        TypeRef::String => Ok(format!(
            "{indent}write_varint({value_expr}.len() as u64, out);\n\
             {indent}let mut previous_key: Option<&str> = None;\n\
             {indent}for value in &{value_expr} {{\n\
             {indent}    let key = value.as_str();\n\
             {indent}    if previous_key.is_some_and(|previous| previous >= key) {{\n\
             {indent}        return Err(FcSerializeError::UnsupportedValue {{ type_name: \"Set\", reason: \"set values must be sorted and unique\" }});\n\
             {indent}    }}\n\
             {indent}    previous_key = Some(key);\n\
             {indent}    value.fc_serialize(out)?;\n\
             {indent}}}\n"
        )),
        TypeRef::TimePointSec => Ok(format!(
            "{indent}write_varint({value_expr}.len() as u64, out);\n\
             {indent}let mut previous_key: Option<u32> = None;\n\
             {indent}for value in &{value_expr} {{\n\
             {indent}    let key = open_graphene_fc::parse_time_point_sec(value)?;\n\
             {indent}    if previous_key.is_some_and(|previous| previous >= key) {{\n\
             {indent}        return Err(FcSerializeError::UnsupportedValue {{ type_name: \"Set\", reason: \"set values must be sorted and unique\" }});\n\
             {indent}    }}\n\
             {indent}    previous_key = Some(key);\n\
             {indent}    write_time_point_sec(value, out)?;\n\
             {indent}}}\n"
        )),
        TypeRef::FixedBytes { bytes } => Ok(format!(
            "{indent}write_varint({value_expr}.len() as u64, out);\n\
             {indent}let mut previous_key: Option<Vec<u8>> = None;\n\
             {indent}for value in &{value_expr} {{\n\
             {indent}    let mut key_bytes = Vec::new();\n\
             {indent}    write_fixed_bytes(value, {bytes}, {}, &mut key_bytes)?;\n\
             {indent}    if previous_key.as_ref().is_some_and(|previous| previous >= &key_bytes) {{\n\
             {indent}        return Err(FcSerializeError::UnsupportedValue {{ type_name: \"Set\", reason: \"set values must be sorted and unique\" }});\n\
             {indent}    }}\n\
             {indent}    previous_key = Some(key_bytes.clone());\n\
             {indent}    out.extend_from_slice(&key_bytes);\n\
             {indent}}}\n",
            rust_string_literal(&format!("fixed_bytes_{bytes}"))
        )),
        TypeRef::ProtocolObjectId { .. } => Ok(format!(
            "{indent}write_varint({value_expr}.len() as u64, out);\n\
             {indent}let mut previous_key: Option<u64> = None;\n\
             {indent}for value in &{value_expr} {{\n\
             {indent}    let key_parts = parse_protocol_object_id(&value.0, None, None)?;\n\
             {indent}    if previous_key.is_some_and(|previous| previous >= key_parts.instance) {{\n\
             {indent}        return Err(FcSerializeError::UnsupportedValue {{ type_name: \"Set\", reason: \"set values must be sorted and unique\" }});\n\
             {indent}    }}\n\
             {indent}    previous_key = Some(key_parts.instance);\n\
             {indent}    value.fc_serialize(out)?;\n\
             {indent}}}\n"
        )),
        TypeRef::PublicKey { .. } => {
            let prefix = render_public_key_prefix_expr(inner)?;
            Ok(format!(
                "{indent}write_varint({value_expr}.len() as u64, out);\n\
                 {indent}let mut previous_key: Option<Vec<u8>> = None;\n\
                 {indent}for value in &{value_expr} {{\n\
                 {indent}    let mut key_bytes = Vec::new();\n\
                 {indent}    write_public_key(value, {prefix}, &mut key_bytes)?;\n\
                 {indent}    if previous_key.as_ref().is_some_and(|previous| previous >= &key_bytes) {{\n\
                 {indent}        return Err(FcSerializeError::UnsupportedValue {{ type_name: \"Set\", reason: \"set values must be sorted and unique\" }});\n\
                 {indent}    }}\n\
                 {indent}    previous_key = Some(key_bytes.clone());\n\
                 {indent}    out.extend_from_slice(&key_bytes);\n\
                 {indent}}}\n"
            ))
        }
        _ => Err(GenBindingsRsError::Render {
            message: "internal error: unsupported set value type".to_string(),
        }),
    }
}

fn render_fc_ordered_copy_set_serialize_lines(
    value_expr: &str,
    rust_key_type: &str,
    indent: &str,
) -> Result<String> {
    Ok(format!(
        "{indent}write_varint({value_expr}.len() as u64, out);\n\
         {indent}let mut previous_key: Option<{rust_key_type}> = None;\n\
         {indent}for value in &{value_expr} {{\n\
         {indent}    if previous_key.is_some_and(|previous| previous >= *value) {{\n\
         {indent}        return Err(FcSerializeError::UnsupportedValue {{ type_name: \"Set\", reason: \"set values must be sorted and unique\" }});\n\
         {indent}    }}\n\
         {indent}    previous_key = Some(*value);\n\
         {indent}    value.fc_serialize(out)?;\n\
         {indent}}}\n"
    ))
}

fn render_fc_static_variant_set_serialize_lines(
    value_expr: &str,
    inner: &TypeRef,
    indent: &str,
) -> Result<String> {
    let tag_fn = if is_fee_parameters_type(inner) {
        "fee_parameters_tag"
    } else if is_future_extensions_type(inner) {
        "future_extensions_tag"
    } else {
        return Err(GenBindingsRsError::Render {
            message: "internal error: unsupported static variant set type".to_string(),
        });
    };

    Ok(format!(
        "{indent}write_varint({value_expr}.len() as u64, out);\n\
         {indent}let mut previous_key: Option<u64> = None;\n\
         {indent}for value in &{value_expr} {{\n\
         {indent}    let key = {tag_fn}(value);\n\
         {indent}    if previous_key.is_some_and(|previous| previous >= key) {{\n\
         {indent}        return Err(FcSerializeError::UnsupportedValue {{ type_name: \"Set\", reason: \"set values must be sorted and unique\" }});\n\
         {indent}    }}\n\
         {indent}    previous_key = Some(key);\n\
         {indent}    value.fc_serialize(out)?;\n\
         {indent}}}\n"
    ))
}

fn render_fc_flat_map_serialize_lines(
    value_expr: &str,
    key: &TypeRef,
    value: &TypeRef,
    indent: &str,
) -> Result<String> {
    if matches!(key, TypeRef::Address) && matches!(value, TypeRef::Uint16) {
        return Ok(format!(
            "{indent}if !{value_expr}.is_empty() {{\n\
             {indent}    return Err(FcSerializeError::UnsupportedValue {{ type_name: \"Address\", reason: \"address flat_map FC serialization is not implemented\" }});\n\
             {indent}}}\n\
             {indent}write_varint(0, out);\n"
        ));
    }

    if !is_fc_supported_flat_map(key, value) {
        return Err(GenBindingsRsError::Render {
            message: "internal error: unsupported flat_map key/value type".to_string(),
        });
    }

    match key {
        TypeRef::ProtocolObjectId { .. } => Ok(format!(
            "{indent}write_varint({value_expr}.len() as u64, out);\n\
             {indent}let mut previous_key: Option<u64> = None;\n\
             {indent}for (key, value) in &{value_expr} {{\n\
             {indent}    let key_parts = parse_protocol_object_id(&key.0, None, None)?;\n\
             {indent}    if previous_key.is_some_and(|previous| previous >= key_parts.instance) {{\n\
             {indent}        return Err(FcSerializeError::UnsupportedValue {{ type_name: \"FlatMap\", reason: \"flat_map keys must be sorted and unique\" }});\n\
             {indent}    }}\n\
             {indent}    previous_key = Some(key_parts.instance);\n\
             {indent}    key.fc_serialize(out)?;\n\
             {indent}    value.fc_serialize(out)?;\n\
             {indent}}}\n"
        )),
        TypeRef::PublicKey { .. } => {
            let prefix = render_public_key_prefix_expr(key)?;
            Ok(format!(
                "{indent}write_varint({value_expr}.len() as u64, out);\n\
                 {indent}let mut previous_key: Option<Vec<u8>> = None;\n\
                 {indent}for (key, value) in &{value_expr} {{\n\
                 {indent}    let mut key_bytes = Vec::new();\n\
                 {indent}    write_public_key(key, {prefix}, &mut key_bytes)?;\n\
                 {indent}    if previous_key.as_ref().is_some_and(|previous| previous >= &key_bytes) {{\n\
                 {indent}        return Err(FcSerializeError::UnsupportedValue {{ type_name: \"FlatMap\", reason: \"flat_map keys must be sorted and unique\" }});\n\
                 {indent}    }}\n\
                 {indent}    previous_key = Some(key_bytes.clone());\n\
                 {indent}    out.extend_from_slice(&key_bytes);\n\
                 {indent}    value.fc_serialize(out)?;\n\
                 {indent}}}\n"
            ))
        }
        _ => Err(GenBindingsRsError::Render {
            message: "internal error: unsupported flat_map key type".to_string(),
        }),
    }
}

fn is_fc_supported_set(inner: &TypeRef) -> bool {
    matches!(
        inner,
        TypeRef::Bool
            | TypeRef::Uint16
            | TypeRef::Uint32
            | TypeRef::Int32 { .. }
            | TypeRef::Int64 { json: None, .. }
            | TypeRef::Uint64 { json: None, .. }
            | TypeRef::String
            | TypeRef::TimePointSec
            | TypeRef::FixedBytes { .. }
            | TypeRef::ProtocolObjectId { .. }
            | TypeRef::PublicKey { .. }
    )
}

fn is_fee_parameters_type(ty: &TypeRef) -> bool {
    matches!(ty, TypeRef::StaticVariantRef { name } if name == "fee_parameters")
}

fn is_future_extensions_type(ty: &TypeRef) -> bool {
    matches!(ty, TypeRef::StaticVariantRef { name } if name == "future_extensions")
}

fn is_fc_supported_flat_map(key: &TypeRef, value: &TypeRef) -> bool {
    if matches!(key, TypeRef::Address) {
        return matches!(value, TypeRef::Uint16);
    }

    matches!(
        key,
        TypeRef::ProtocolObjectId { .. } | TypeRef::PublicKey { .. }
    ) && (matches!(value, TypeRef::Uint16 | TypeRef::Int64 { json: None, .. })
        || matches!(value, TypeRef::Ref { name } if name == "price"))
}

fn render_vote_id_arg(value_expr: &str, ty: &TypeRef) -> Result<String> {
    match ty {
        TypeRef::VoteId => Ok(format!("&{value_expr}")),
        TypeRef::ProtocolObjectId { object_type } if object_type == "vote" => {
            Ok(format!("&{value_expr}.0"))
        }
        _ => Err(GenBindingsRsError::Render {
            message: "internal error: expected vote id type".to_string(),
        }),
    }
}

fn render_public_key_prefix_expr(ty: &TypeRef) -> Result<String> {
    match ty {
        TypeRef::PublicKey {
            chain_prefix,
            prefix_ref,
        } => {
            if let Some(prefix) = chain_prefix {
                Ok(format!("Some({})", rust_string_literal(prefix)))
            } else if prefix_ref.as_deref() == Some("chain.publicKeyPrefix") {
                Ok("Some(crate::generated::ids::PUBLIC_KEY_PREFIX)".to_string())
            } else if let Some(prefix_ref) = prefix_ref {
                Err(GenBindingsRsError::Render {
                    message: format!("unsupported public key prefix reference `{prefix_ref}`"),
                })
            } else {
                Ok("None".to_string())
            }
        }
        _ => Err(GenBindingsRsError::Render {
            message: "internal error: expected public key type".to_string(),
        }),
    }
}

fn render_fc_transfer_operation_impl(out: &mut String) {
    out.push_str("impl FcSerialize for crate::generated::operations::TransferOperation {\n");
    out.push_str("    fn fc_serialize(&self, out: &mut Vec<u8>) -> Result<()> {\n");
    out.push_str("        self.fee.fc_serialize(out)?;\n");
    out.push_str("        self.from.fc_serialize(out)?;\n");
    out.push_str("        self.to.fc_serialize(out)?;\n");
    out.push_str("        self.amount.fc_serialize(out)?;\n");
    out.push_str("        if self.memo.is_some() {\n");
    out.push_str("            return Err(FcSerializeError::UnsupportedValue { type_name: \"MemoData\", reason: \"memo FC serialization is not implemented in the minimal transfer slice\" });\n");
    out.push_str("        }\n");
    out.push_str("        out.push(0);\n");
    out.push_str("        self.extensions.fc_serialize(out)?;\n");
    out.push_str("        Ok(())\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");
}

fn is_fc_supported_operation(
    protocol: &Protocol,
    operation: &OperationDef,
    supported_structs: &BTreeSet<String>,
) -> bool {
    operation
        .fields
        .iter()
        .all(|field| is_fc_supported_type(protocol, &field.ty, supported_structs))
}

fn is_fc_supported_type(
    protocol: &Protocol,
    ty: &TypeRef,
    supported_structs: &BTreeSet<String>,
) -> bool {
    match ty {
        TypeRef::Void
        | TypeRef::Bool
        | TypeRef::Uint8
        | TypeRef::Uint16
        | TypeRef::Uint32
        | TypeRef::Int32 { .. }
        | TypeRef::Int64 { json: None, .. }
        | TypeRef::Uint64 { json: None, .. }
        | TypeRef::String
        | TypeRef::Bytes
        | TypeRef::FixedBytes { .. }
        | TypeRef::PublicKey { .. }
        | TypeRef::TimePointSec
        | TypeRef::VoteId
        | TypeRef::Signature => true,
        TypeRef::ObjectId | TypeRef::ProtocolObjectId { .. } => true,
        TypeRef::Ref { name } => supported_structs.contains(name),
        TypeRef::StaticVariantRef { name } => {
            name == "future_extensions"
                || name == "special_authority"
                || name == "htlc_hash"
                || name == "predicate"
                || name == "vesting_policy_initializer"
                || name == "worker_initializer"
                || name == "limit_order_auto_action"
                || name == "fee_parameters"
                || name == "argument_type"
                || name == "operation"
        }
        TypeRef::Optional { inner } | TypeRef::Vector { inner } => {
            is_fc_supported_type(protocol, inner, supported_structs)
        }
        TypeRef::Set { inner, .. } if is_vote_id_type(inner) => true,
        TypeRef::Set { inner, .. } if is_fee_parameters_type(inner) => true,
        TypeRef::Set { inner, .. }
            if is_fc_supported_set(inner)
                || is_fee_parameters_type(inner)
                || is_future_extensions_type(inner) =>
        {
            true
        }
        TypeRef::FlatMap { key, value, .. } if is_fc_supported_flat_map(key, value) => true,
        TypeRef::Pair { first, second } => {
            is_fc_supported_type(protocol, first, supported_structs)
                && is_fc_supported_type(protocol, second, supported_structs)
        }
        TypeRef::Set { .. }
        | TypeRef::Map { .. }
        | TypeRef::FlatMap { .. }
        | TypeRef::Int64 { .. }
        | TypeRef::Uint64 { .. }
        | TypeRef::Uint128 { .. }
        | TypeRef::UnsignedVarint
        | TypeRef::CallbackHandle
        | TypeRef::FixedHex { .. }
        | TypeRef::TimePoint
        | TypeRef::Address
        | TypeRef::ProtocolObjectUnion { .. }
        | TypeRef::AnyJson { .. }
        | TypeRef::Unsupported { .. } => false,
    }
}

fn render_static_variant(
    out: &mut String,
    protocol: &Protocol,
    variant: &StaticVariantDef,
    no_recursion_cuts: &SchemaNoRecursionCuts,
) -> Result<()> {
    let enum_name = rust_type_name(&variant.name);
    out.push_str(&format!(
        "/// Static variant `{}` serialized as Graphene `[tag, value]`.\n",
        variant.name
    ));
    out.push_str("#[derive(Debug, Clone, PartialEq, utoipa::ToSchema)]\n");
    out.push_str(&format!(
        "#[schema(as = {})]\n",
        openapi_schema_name(protocol, &enum_name)
    ));
    out.push_str(&format!("pub enum {enum_name} {{\n"));

    let mut variants = variant.variants.clone();
    variants.sort_by_key(|arm| arm.tag);
    let mut emitted = BTreeSet::new();
    let mut method_names = BTreeSet::new();
    let mut rendered_arms = Vec::new();
    for arm in variants {
        let variant_name = rust_variant_name(&arm.name);
        ensure_unique(&mut emitted, &variant_name, "static variant arm")?;
        let method_name =
            static_variant_constructor_name(&enum_name, &arm.name, &mut method_names)?;
        let ty = render_type_ref(protocol, &arm.ty)?;
        if no_recursion_cuts.variants.contains(&SchemaVariantKey {
            owner: enum_name.clone(),
            variant: variant_name.clone(),
        }) {
            out.push_str("    #[schema(no_recursion)]\n");
        }
        out.push_str(&format!("    {variant_name}(Box<{ty}>),\n"));
        rendered_arms.push((arm.tag, variant_name, ty, method_name));
    }

    out.push_str("}\n\n");
    if enum_name == "Operation" {
        render_static_variant_constructor_impl(out, &enum_name, &rendered_arms);
        render_static_variant_accessor_impl(out, &enum_name, &rendered_arms);
    }
    if enum_name == "FutureExtensions" {
        render_future_extensions_empty_impl(out, &rendered_arms);
    }
    render_static_variant_serialize_impl(out, &enum_name, &rendered_arms);
    render_static_variant_deserialize_impl(out, &enum_name, &rendered_arms);
    Ok(())
}

fn render_static_variant_constructor_impl(
    out: &mut String,
    enum_name: &str,
    arms: &[(u32, String, String, String)],
) {
    out.push_str(&format!("impl {enum_name} {{\n"));
    for (index, (_, variant_name, ty, method_name)) in arms.iter().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        let signature = format!("    pub fn {method_name}(value: {ty}) -> Self {{");
        if signature.len() <= 100 {
            out.push_str(&format!(
                "{signature}\n        Self::{variant_name}(Box::new(value))\n    }}\n"
            ));
        } else {
            out.push_str(&format!(
                "    pub fn {method_name}(\n        value: {ty},\n    ) -> Self {{\n        Self::{variant_name}(Box::new(value))\n    }}\n"
            ));
        }
    }
    out.push_str("}\n\n");
}

fn render_static_variant_accessor_impl(
    out: &mut String,
    enum_name: &str,
    arms: &[(u32, String, String, String)],
) {
    out.push_str(&format!("impl {enum_name} {{\n"));
    for (_, variant_name, ty, method_name) in arms {
        out.push_str(&format!(
            "    pub fn as_{method_name}(&self) -> Option<&{ty}> {{\n        match self {{\n            Self::{variant_name}(value) => Some(value.as_ref()),\n            _ => None,\n        }}\n    }}\n\n"
        ));
    }
    out.push_str("}\n\n");
}

fn render_future_extensions_empty_impl(out: &mut String, arms: &[(u32, String, String, String)]) {
    if arms
        .iter()
        .any(|(_, variant_name, ty, _)| variant_name == "VoidT" && ty == "()")
    {
        out.push_str("impl FutureExtensions {\n");
        out.push_str("    pub fn empty() -> Self {\n");
        out.push_str("        Self::VoidT(Box::new(()))\n");
        out.push_str("    }\n");
        out.push_str("}\n\n");
    }
}

fn render_static_variant_serialize_impl(
    out: &mut String,
    enum_name: &str,
    arms: &[(u32, String, String, String)],
) {
    out.push_str(&format!("impl serde::Serialize for {enum_name} {{\n"));
    out.push_str("    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>\n");
    out.push_str("    where\n");
    out.push_str("        S: serde::Serializer,\n");
    out.push_str("    {\n");
    out.push_str("        use serde::ser::SerializeSeq;\n");
    out.push_str("        let mut seq = serializer.serialize_seq(Some(2))?;\n");
    out.push_str("        match self {\n");
    for (tag, variant_name, ty, _) in arms {
        // Byte payloads (hashes etc.) are hex strings on the wire, not JSON number arrays.
        let value_element = if ty == "Vec<u8>" {
            "seq.serialize_element(&open_graphene_core::bytes_to_hex(value.as_ref()))?;"
        } else {
            "seq.serialize_element(value.as_ref())?;"
        };
        out.push_str(&format!(
            "            Self::{variant_name}(value) => {{\n                seq.serialize_element(&{tag}u32)?;\n                {value_element}\n            }}\n"
        ));
    }
    out.push_str("        }\n");
    out.push_str("        seq.end()\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");
}

fn render_static_variant_deserialize_impl(
    out: &mut String,
    enum_name: &str,
    arms: &[(u32, String, String, String)],
) {
    out.push_str(&format!(
        "impl<'de> serde::Deserialize<'de> for {enum_name} {{\n"
    ));
    out.push_str("    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>\n");
    out.push_str("    where\n");
    out.push_str("        D: serde::Deserializer<'de>,\n");
    out.push_str("    {\n");
    out.push_str(
        "        let mut values = <Vec<serde_json::Value> as serde::Deserialize>::deserialize(deserializer)?;\n",
    );
    out.push_str("        if values.len() != 2 {\n");
    out.push_str(&format!(
        "            return Err(serde::de::Error::custom(format!(\"expected static variant {enum_name} as [tag, value], got {{}} elements\", values.len())));\n"
    ));
    out.push_str("        }\n");
    out.push_str("        let payload = values.pop().expect(\"length checked\");\n");
    out.push_str("        let tag_value = values.pop().expect(\"length checked\");\n");
    out.push_str("        let tag = tag_value\n");
    out.push_str("            .as_u64()\n");
    out.push_str(&format!(
        "            .ok_or_else(|| serde::de::Error::custom(\"expected numeric tag for static variant {enum_name}\"))? as u32;\n"
    ));
    out.push_str("        match tag {\n");
    for (tag, variant_name, ty, _) in arms {
        // Byte payloads arrive as hex strings (or arrays); decode both.
        if ty == "Vec<u8>" {
            out.push_str(&format!(
                "            {tag} => open_graphene_core::deserialize_bytes_from_hex_string_or_byte_array(payload)\n                .map(|value| Self::{variant_name}(Box::new(value)))\n                .map_err(serde::de::Error::custom),\n"
            ));
        } else {
            out.push_str(&format!(
                "            {tag} => serde_json::from_value::<{ty}>(payload)\n                .map(|value| Self::{variant_name}(Box::new(value)))\n                .map_err(serde::de::Error::custom),\n"
            ));
        }
    }
    out.push_str(&format!(
        "            other => Err(serde::de::Error::custom(format!(\"unknown static variant {enum_name} tag {{other}}\"))),\n"
    ));
    out.push_str("        }\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");
}

fn static_variant_constructor_name(
    enum_name: &str,
    arm_name: &str,
    emitted: &mut BTreeSet<String>,
) -> Result<String> {
    let preferred_name = if enum_name == "Operation" {
        arm_name.strip_suffix("_operation").unwrap_or(arm_name)
    } else {
        arm_name
    };
    let mut method_name = rust_field_name(preferred_name);
    if !emitted.insert(method_name.clone()) {
        method_name = rust_field_name(arm_name);
        ensure_unique(emitted, &method_name, "static variant constructor")?;
    }
    Ok(method_name)
}

fn render_type_ref(protocol: &Protocol, ty: &TypeRef) -> Result<String> {
    Ok(match ty {
        TypeRef::Void => "()".to_string(),
        TypeRef::Bool => "bool".to_string(),
        TypeRef::Uint8 => "u8".to_string(),
        TypeRef::Uint16 => "u16".to_string(),
        TypeRef::Uint32 => "u32".to_string(),
        TypeRef::Int32 { .. } => "i32".to_string(),
        TypeRef::Int64 { json, .. }
            if json == &Some(JsonShape::DecimalString) || json == &Some(JsonShape::String) =>
        {
            "String".to_string()
        }
        TypeRef::Int64 { .. } => "i64".to_string(),
        TypeRef::Uint64 { json, .. }
            if json == &Some(JsonShape::DecimalString) || json == &Some(JsonShape::String) =>
        {
            "String".to_string()
        }
        TypeRef::Uint64 { .. } => "u64".to_string(),
        TypeRef::Uint128 { .. } => "String".to_string(),
        TypeRef::UnsignedVarint | TypeRef::CallbackHandle => "u64".to_string(),
        TypeRef::String
        | TypeRef::FixedHex { .. }
        | TypeRef::TimePointSec
        | TypeRef::TimePoint
        | TypeRef::PublicKey { .. }
        | TypeRef::Address
        | TypeRef::VoteId => "String".to_string(),
        TypeRef::Signature => "crate::generated::types::Signature".to_string(),
        TypeRef::Bytes | TypeRef::FixedBytes { .. } => "Vec<u8>".to_string(),
        TypeRef::ObjectId => "crate::generated::ids::ObjectId".to_string(),
        TypeRef::ProtocolObjectId { object_type } => {
            format!(
                "crate::generated::ids::{}",
                object_id_type_name(object_type)
            )
        }
        TypeRef::ProtocolObjectUnion { .. } | TypeRef::AnyJson { .. } => {
            "serde_json::Value".to_string()
        }
        TypeRef::Optional { inner } => format!("Option<{}>", render_type_ref(protocol, inner)?),
        TypeRef::Vector { inner } | TypeRef::Set { inner, .. } => {
            format!("Vec<{}>", render_type_ref(protocol, inner)?)
        }
        TypeRef::Map { key, value, .. } | TypeRef::FlatMap { key, value, .. } => format!(
            "Vec<({}, {})>",
            render_type_ref(protocol, key)?,
            render_type_ref(protocol, value)?
        ),
        TypeRef::Pair { first, second } => format!(
            "({}, {})",
            render_type_ref(protocol, first)?,
            render_type_ref(protocol, second)?
        ),
        TypeRef::Ref { name } if is_operation_ref(protocol, name) => {
            format!("crate::generated::operations::{}", rust_type_name(name))
        }
        TypeRef::Ref { name } => format!("crate::generated::types::{}", rust_type_name(name)),
        TypeRef::StaticVariantRef { name } => {
            format!(
                "crate::generated::static_variants::{}",
                rust_type_name(name)
            )
        }
        TypeRef::Unsupported { reason, .. } => {
            return Err(GenBindingsRsError::Render {
                message: format!("unsupported TypeRef in bindings generator: {reason}"),
            });
        }
    })
}

fn is_operation_ref(protocol: &Protocol, name: &str) -> bool {
    protocol
        .operations
        .iter()
        .any(|operation| operation.name == name)
        || protocol
            .structs
            .iter()
            .any(|struct_def| struct_def.name == name && struct_def.kind == StructKind::Operation)
}

fn protocol_uses_signature(protocol: &Protocol) -> bool {
    protocol.structs.iter().any(|struct_def| {
        struct_def
            .fields
            .iter()
            .any(|field| type_uses_signature(&field.ty))
    }) || protocol.operations.iter().any(|operation| {
        operation
            .fields
            .iter()
            .any(|field| type_uses_signature(&field.ty))
    }) || protocol.static_variants.iter().any(|variant| {
        variant
            .variants
            .iter()
            .any(|arm| type_uses_signature(&arm.ty))
    })
}

fn type_uses_signature(ty: &TypeRef) -> bool {
    match ty {
        TypeRef::Signature => true,
        TypeRef::Optional { inner } | TypeRef::Vector { inner } | TypeRef::Set { inner, .. } => {
            type_uses_signature(inner)
        }
        TypeRef::Map { key, value, .. } | TypeRef::FlatMap { key, value, .. } => {
            type_uses_signature(key) || type_uses_signature(value)
        }
        TypeRef::Pair { first, second } => {
            type_uses_signature(first) || type_uses_signature(second)
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
        | TypeRef::ObjectId
        | TypeRef::ProtocolObjectId { .. }
        | TypeRef::ProtocolObjectUnion { .. }
        | TypeRef::VoteId
        | TypeRef::Ref { .. }
        | TypeRef::StaticVariantRef { .. }
        | TypeRef::AnyJson { .. }
        | TypeRef::Unsupported { .. } => false,
    }
}

fn protocol_uses_fixed_bytes(protocol: &Protocol) -> bool {
    !fixed_byte_lengths(protocol).is_empty()
}

fn fixed_byte_lengths(protocol: &Protocol) -> BTreeSet<usize> {
    let mut lengths = BTreeSet::new();

    for struct_def in &protocol.structs {
        for field in &struct_def.fields {
            collect_fixed_byte_lengths_from_type(&field.ty, &mut lengths);
        }
    }
    for operation in &protocol.operations {
        for field in &operation.fields {
            collect_fixed_byte_lengths_from_type(&field.ty, &mut lengths);
        }
    }
    for static_variant in &protocol.static_variants {
        for arm in &static_variant.variants {
            collect_fixed_byte_lengths_from_type(&arm.ty, &mut lengths);
        }
    }
    for method in &protocol.rpc_methods {
        for param in &method.params {
            collect_fixed_byte_lengths_from_type(&param.ty, &mut lengths);
        }
        if let Some(returns) = &method.returns {
            collect_fixed_byte_lengths_from_type(returns, &mut lengths);
        }
        for notice in &method.notices {
            if let Some(payload) = &notice.payload {
                collect_fixed_byte_lengths_from_type(payload, &mut lengths);
            }
        }
    }

    lengths
}

fn collect_fixed_byte_lengths_from_type(ty: &TypeRef, lengths: &mut BTreeSet<usize>) {
    match ty {
        TypeRef::FixedBytes { bytes } => {
            lengths.insert(*bytes as usize);
        }
        TypeRef::Optional { inner } | TypeRef::Vector { inner } | TypeRef::Set { inner, .. } => {
            collect_fixed_byte_lengths_from_type(inner, lengths);
        }
        TypeRef::Map { key, value, .. } | TypeRef::FlatMap { key, value, .. } => {
            collect_fixed_byte_lengths_from_type(key, lengths);
            collect_fixed_byte_lengths_from_type(value, lengths);
        }
        TypeRef::Pair { first, second } => {
            collect_fixed_byte_lengths_from_type(first, lengths);
            collect_fixed_byte_lengths_from_type(second, lengths);
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
        | TypeRef::TimePointSec
        | TypeRef::TimePoint
        | TypeRef::PublicKey { .. }
        | TypeRef::Address
        | TypeRef::Signature
        | TypeRef::ObjectId
        | TypeRef::ProtocolObjectId { .. }
        | TypeRef::ProtocolObjectUnion { .. }
        | TypeRef::VoteId
        | TypeRef::Ref { .. }
        | TypeRef::StaticVariantRef { .. }
        | TypeRef::AnyJson { .. }
        | TypeRef::Unsupported { .. } => {}
    }
}

fn collect_protocol_object_id_names(protocol: &Protocol) -> BTreeSet<String> {
    let mut names = BTreeSet::new();

    for struct_def in &protocol.structs {
        for field in &struct_def.fields {
            collect_protocol_object_id_names_from_type(&field.ty, &mut names);
        }
    }
    for operation in &protocol.operations {
        for field in &operation.fields {
            collect_protocol_object_id_names_from_type(&field.ty, &mut names);
        }
    }
    for static_variant in &protocol.static_variants {
        for arm in &static_variant.variants {
            collect_protocol_object_id_names_from_type(&arm.ty, &mut names);
        }
    }
    for method in &protocol.rpc_methods {
        for param in &method.params {
            collect_protocol_object_id_names_from_type(&param.ty, &mut names);
        }
        if let Some(returns) = &method.returns {
            collect_protocol_object_id_names_from_type(returns, &mut names);
        }
        for notice in &method.notices {
            if let Some(payload) = &notice.payload {
                collect_protocol_object_id_names_from_type(payload, &mut names);
            }
        }
    }

    names
}

fn collect_protocol_object_id_names_from_type(ty: &TypeRef, names: &mut BTreeSet<String>) {
    match ty {
        TypeRef::ProtocolObjectId { object_type } => {
            names.insert(object_type.clone());
        }
        TypeRef::Optional { inner } | TypeRef::Vector { inner } | TypeRef::Set { inner, .. } => {
            collect_protocol_object_id_names_from_type(inner, names);
        }
        TypeRef::Map { key, value, .. } | TypeRef::FlatMap { key, value, .. } => {
            collect_protocol_object_id_names_from_type(key, names);
            collect_protocol_object_id_names_from_type(value, names);
        }
        TypeRef::Pair { first, second } => {
            collect_protocol_object_id_names_from_type(first, names);
            collect_protocol_object_id_names_from_type(second, names);
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
        | TypeRef::ProtocolObjectUnion { .. }
        | TypeRef::VoteId
        | TypeRef::Ref { .. }
        | TypeRef::StaticVariantRef { .. }
        | TypeRef::AnyJson { .. }
        | TypeRef::Unsupported { .. } => {}
    }
}

fn generated_header(protocol: &Protocol, module: &str) -> String {
    format!(
        "// Generated by open-graphene-gen-bindings-rs.\n\
         // Chain: {} | schema version: {} | module: {module}.\n\
         // Do not edit by hand.\n\n",
        protocol.chain.id, protocol.schema_version
    )
}

fn sorted_enums(enums: &[EnumDef]) -> Vec<EnumDef> {
    let mut enums = enums.to_vec();
    enums.sort_by(|a, b| a.name.cmp(&b.name));
    enums
}

fn sorted_structs(structs: &[StructDef]) -> Vec<StructDef> {
    let mut structs = structs.to_vec();
    structs.sort_by(|a, b| a.name.cmp(&b.name));
    structs
}

fn sorted_operations(operations: &[OperationDef]) -> Vec<OperationDef> {
    let mut operations = operations.to_vec();
    operations.sort_by_key(|operation| operation.wire_tag);
    operations
}

fn sorted_static_variants(static_variants: &[StaticVariantDef]) -> Vec<StaticVariantDef> {
    let mut static_variants = static_variants.to_vec();
    static_variants.sort_by(|a, b| a.name.cmp(&b.name));
    static_variants
}

fn ensure_unique(seen: &mut BTreeSet<String>, name: &str, scope: &str) -> Result<()> {
    if seen.insert(name.to_string()) {
        Ok(())
    } else {
        Err(GenBindingsRsError::Render {
            message: format!("duplicate generated {scope} name `{name}`"),
        })
    }
}

fn object_id_type_name(object_type: &str) -> String {
    format!("{}Id", rust_type_name(object_type))
}

fn openapi_schema_name(protocol: &Protocol, rust_name: &str) -> String {
    format!(
        "Graphene{}{}",
        rust_type_name(&protocol.chain.id),
        rust_name
    )
}

fn rust_type_name(value: &str) -> String {
    let words = words(value);
    let mut out = String::new();
    for word in words {
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            out.extend(first.to_uppercase());
            out.push_str(chars.as_str());
        }
    }
    if out.is_empty() {
        "GeneratedType".to_string()
    } else if out.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        format!("N{out}")
    } else {
        out
    }
}

fn rust_variant_name(value: &str) -> String {
    let name = rust_type_name(value);
    if is_rust_keyword(&name) {
        format!("{name}Variant")
    } else {
        name
    }
}

fn rust_const_name(value: &str) -> String {
    let words = words(value);
    let mut out = words
        .into_iter()
        .map(|word| word.to_ascii_uppercase())
        .collect::<Vec<_>>()
        .join("_");
    if out.is_empty() {
        out.push_str("GENERATED_CONST");
    }
    if out.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        out.insert(0, 'N');
    }
    out
}

fn rust_field_name(value: &str) -> String {
    let words = words(value);
    let mut out = words.join("_");
    if out.is_empty() {
        out.push_str("generated_field");
    }
    if out.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        out.insert(0, '_');
    }
    if is_rust_keyword(&out) {
        format!("r#{out}")
    } else {
        out
    }
}

fn words(value: &str) -> Vec<String> {
    value
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(|part| part.to_ascii_lowercase())
        .collect()
}

fn serde_rename_attr(indent: &str, original: &str, generated: &str) -> String {
    let normalized_generated = generated.strip_prefix("r#").unwrap_or(generated);
    if original == normalized_generated {
        String::new()
    } else {
        format!(
            "{indent}#[serde(rename = {})]\n",
            rust_string_literal(original)
        )
    }
}

fn is_rust_keyword(value: &str) -> bool {
    matches!(
        value,
        "as" | "break"
            | "const"
            | "continue"
            | "crate"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "Self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
            | "async"
            | "await"
            | "dyn"
            | "abstract"
            | "become"
            | "box"
            | "do"
            | "final"
            | "macro"
            | "override"
            | "priv"
            | "try"
            | "typeof"
            | "unsized"
            | "virtual"
            | "yield"
    )
}

fn rust_string_literal(value: &str) -> String {
    serde_json::to_string(value).expect("serializing a string literal cannot fail")
}

#[cfg(test)]
mod tests {
    use super::*;
    use open_graphene_json_schema::types::OrderingRule;
    use open_graphene_json_schema::{ChainDef, FieldDef, OperationDef};

    #[test]
    fn renders_module_index_and_metadata_constants() {
        let protocol = minimal_protocol();
        let output = render_mod(&protocol).expect("render mod");
        let ids = render_ids(&protocol).expect("render ids");

        assert!(output.contains("pub mod ids;"));
        assert!(output.contains("pub use types::*;"));
        assert!(ids.contains("pub const CHAIN_ID: &str = \"swaplock\";"));
        assert!(ids.contains("pub const RPC_METHOD_COUNT: usize = 0;"));
        assert!(ids.contains("impl ObjectId"));
        assert!(ids.contains("pub fn new(value: impl Into<String>) -> Self"));
        assert!(ids.contains("impl From<String> for ObjectId"));
        assert!(ids.contains("impl From<&str> for ObjectId"));
    }

    #[test]
    fn renders_operation_refs_to_operations_module() {
        let mut protocol = minimal_protocol();
        protocol.operations.push(OperationDef {
            name: "transfer_operation".to_string(),
            wire_tag: 0,
            fields: vec![],
            is_virtual: false,
            source: None,
            support: None,
        });

        let rendered = render_type_ref(
            &protocol,
            &TypeRef::Ref {
                name: "transfer_operation".to_string(),
            },
        )
        .expect("render ref");

        assert_eq!(rendered, "crate::generated::operations::TransferOperation");
    }

    #[test]
    fn rust_names_handle_keywords_and_protocol_case() {
        assert_eq!(
            rust_type_name("account_create_operation"),
            "AccountCreateOperation"
        );
        assert_eq!(rust_field_name("type"), "r#type");
        assert_eq!(
            rust_const_name("transfer_operation_id"),
            "TRANSFER_OPERATION_ID"
        );
        assert_eq!(object_id_type_name("account"), "AccountId");
    }

    #[test]
    fn fields_are_sorted_by_index() {
        let protocol = minimal_protocol();
        let fields = vec![
            FieldDef {
                index: 1,
                name: "second".to_string(),
                ty: TypeRef::String,
                source: None,
                support: None,
            },
            FieldDef {
                index: 0,
                name: "first".to_string(),
                ty: TypeRef::Bool,
                source: None,
                support: None,
            },
        ];
        let mut out = String::new();
        render_fields(
            &mut out,
            &protocol,
            "TestStruct",
            &fields,
            &SchemaNoRecursionCuts::default(),
        )
        .expect("render fields");

        let first = out.find("pub first").expect("first field");
        let second = out.find("pub second").expect("second field");
        assert!(first < second);
    }

    #[test]
    fn static_variant_renderer_emits_graphene_tagged_tuple_serde() {
        let protocol = minimal_protocol();
        let variant = StaticVariantDef {
            name: "operation".to_string(),
            kind: "static_variant".to_string(),
            json: "tagged_tuple".to_string(),
            fc: "static_variant".to_string(),
            variants: vec![open_graphene_json_schema::StaticVariantArmDef {
                tag: 0,
                name: "transfer_operation".to_string(),
                ty: TypeRef::String,
                support: None,
            }],
            source: None,
            support: None,
        };
        let mut out = String::new();
        render_static_variant(
            &mut out,
            &protocol,
            &variant,
            &SchemaNoRecursionCuts::default(),
        )
        .expect("render static variant");

        assert!(out.contains("impl Operation"));
        assert!(out.contains("pub fn transfer(value: String) -> Self"));
        assert!(out.contains("Self::TransferOperation(Box::new(value))"));
        assert!(out.contains("pub fn as_transfer(&self) -> Option<&String>"));
        assert!(out.contains("Self::TransferOperation(value) => Some(value.as_ref())"));
        assert!(out.contains("impl serde::Serialize for Operation"));
        assert!(out.contains("seq.serialize_element(&0u32)?;"));
        assert!(out.contains("impl<'de> serde::Deserialize<'de> for Operation"));
        assert!(out.contains("0 => serde_json::from_value::<String>(payload)"));
    }

    #[test]
    fn static_variant_renderer_emits_future_extensions_empty_helper() {
        let protocol = minimal_protocol();
        let variant = StaticVariantDef {
            name: "future_extensions".to_string(),
            kind: "static_variant".to_string(),
            json: "tagged_tuple".to_string(),
            fc: "static_variant".to_string(),
            variants: vec![open_graphene_json_schema::StaticVariantArmDef {
                tag: 0,
                name: "void_t".to_string(),
                ty: TypeRef::Void,
                support: None,
            }],
            source: None,
            support: None,
        };
        let mut out = String::new();
        render_static_variant(
            &mut out,
            &protocol,
            &variant,
            &SchemaNoRecursionCuts::default(),
        )
        .expect("render static variant");

        assert!(out.contains("impl FutureExtensions"));
        assert!(out.contains("pub fn empty() -> Self"));
        assert!(out.contains("Self::VoidT(Box::new(()))"));
        assert!(!out.contains("pub fn void_t"));
    }

    #[test]
    fn fc_renderer_emits_transfer_path_impls_and_explicit_unsupported_variants() {
        let mut protocol = minimal_protocol();
        protocol.structs.push(StructDef {
            name: "asset".to_string(),
            source_name: None,
            kind: StructKind::Struct,
            wire_tag: None,
            fields: vec![
                FieldDef {
                    index: 0,
                    name: "amount".to_string(),
                    ty: TypeRef::Int64 {
                        json: None,
                        fc: None,
                    },
                    source: None,
                    support: None,
                },
                FieldDef {
                    index: 1,
                    name: "asset_id".to_string(),
                    ty: TypeRef::ProtocolObjectId {
                        object_type: "asset".to_string(),
                    },
                    source: None,
                    support: None,
                },
            ],
            support: None,
        });
        protocol.structs.push(StructDef {
            name: "price".to_string(),
            source_name: None,
            kind: StructKind::Struct,
            wire_tag: None,
            fields: vec![
                FieldDef {
                    index: 0,
                    name: "base".to_string(),
                    ty: TypeRef::Ref {
                        name: "asset".to_string(),
                    },
                    source: None,
                    support: None,
                },
                FieldDef {
                    index: 1,
                    name: "quote".to_string(),
                    ty: TypeRef::Ref {
                        name: "asset".to_string(),
                    },
                    source: None,
                    support: None,
                },
            ],
            support: None,
        });
        protocol.structs.push(StructDef {
            name: "unsupported_signature_struct".to_string(),
            source_name: None,
            kind: StructKind::Struct,
            wire_tag: None,
            fields: vec![FieldDef {
                index: 0,
                name: "value".to_string(),
                ty: TypeRef::Signature,
                source: None,
                support: None,
            }],
            support: None,
        });
        protocol.structs.push(StructDef {
            name: "no_special_authority".to_string(),
            source_name: None,
            kind: StructKind::Struct,
            wire_tag: None,
            fields: vec![],
            support: None,
        });
        protocol.structs.push(StructDef {
            name: "top_holders_special_authority".to_string(),
            source_name: None,
            kind: StructKind::Struct,
            wire_tag: None,
            fields: vec![
                FieldDef {
                    index: 0,
                    name: "asset".to_string(),
                    ty: TypeRef::ProtocolObjectId {
                        object_type: "asset".to_string(),
                    },
                    source: None,
                    support: None,
                },
                FieldDef {
                    index: 1,
                    name: "num_top_holders".to_string(),
                    ty: TypeRef::Uint8,
                    source: None,
                    support: None,
                },
            ],
            support: None,
        });
        protocol.operations.push(OperationDef {
            name: "transfer_operation".to_string(),
            wire_tag: 0,
            fields: vec![],
            is_virtual: false,
            source: None,
            support: None,
        });
        protocol.operations.push(OperationDef {
            name: "limit_order_create_operation".to_string(),
            wire_tag: 1,
            fields: vec![],
            is_virtual: false,
            source: None,
            support: None,
        });
        protocol.operations.push(OperationDef {
            name: "limit_order_cancel_operation".to_string(),
            wire_tag: 2,
            fields: vec![],
            is_virtual: false,
            source: None,
            support: None,
        });
        protocol.operations.push(OperationDef {
            name: "custom_supported_operation".to_string(),
            wire_tag: 9,
            fields: vec![
                FieldDef {
                    index: 0,
                    name: "fee".to_string(),
                    ty: TypeRef::Ref {
                        name: "asset".to_string(),
                    },
                    source: None,
                    support: None,
                },
                FieldDef {
                    index: 1,
                    name: "enabled".to_string(),
                    ty: TypeRef::Bool,
                    source: None,
                    support: None,
                },
                FieldDef {
                    index: 2,
                    name: "small".to_string(),
                    ty: TypeRef::Uint8,
                    source: None,
                    support: None,
                },
                FieldDef {
                    index: 3,
                    name: "medium".to_string(),
                    ty: TypeRef::Uint16,
                    source: None,
                    support: None,
                },
                FieldDef {
                    index: 4,
                    name: "large".to_string(),
                    ty: TypeRef::Uint32,
                    source: None,
                    support: None,
                },
                FieldDef {
                    index: 5,
                    name: "huge".to_string(),
                    ty: TypeRef::Uint64 {
                        json: None,
                        fc: None,
                    },
                    source: None,
                    support: None,
                },
                FieldDef {
                    index: 6,
                    name: "signed".to_string(),
                    ty: TypeRef::Int32 {
                        fc: None,
                        source: None,
                    },
                    source: None,
                    support: None,
                },
                FieldDef {
                    index: 7,
                    name: "label".to_string(),
                    ty: TypeRef::String,
                    source: None,
                    support: None,
                },
                FieldDef {
                    index: 8,
                    name: "signing_key".to_string(),
                    ty: TypeRef::PublicKey {
                        chain_prefix: None,
                        prefix_ref: Some("chain.publicKeyPrefix".to_string()),
                    },
                    source: None,
                    support: None,
                },
                FieldDef {
                    index: 9,
                    name: "optional_signing_key".to_string(),
                    ty: TypeRef::Optional {
                        inner: Box::new(TypeRef::PublicKey {
                            chain_prefix: None,
                            prefix_ref: Some("chain.publicKeyPrefix".to_string()),
                        }),
                    },
                    source: None,
                    support: None,
                },
                FieldDef {
                    index: 10,
                    name: "expires_at".to_string(),
                    ty: TypeRef::TimePointSec,
                    source: None,
                    support: None,
                },
                FieldDef {
                    index: 11,
                    name: "optional_expiration".to_string(),
                    ty: TypeRef::Optional {
                        inner: Box::new(TypeRef::TimePointSec),
                    },
                    source: None,
                    support: None,
                },
                FieldDef {
                    index: 12,
                    name: "expiration_points".to_string(),
                    ty: TypeRef::Vector {
                        inner: Box::new(TypeRef::TimePointSec),
                    },
                    source: None,
                    support: None,
                },
                FieldDef {
                    index: 13,
                    name: "vote".to_string(),
                    ty: TypeRef::VoteId,
                    source: None,
                    support: None,
                },
                FieldDef {
                    index: 14,
                    name: "optional_vote".to_string(),
                    ty: TypeRef::Optional {
                        inner: Box::new(TypeRef::VoteId),
                    },
                    source: None,
                    support: None,
                },
                FieldDef {
                    index: 15,
                    name: "votes".to_string(),
                    ty: TypeRef::Vector {
                        inner: Box::new(TypeRef::VoteId),
                    },
                    source: None,
                    support: None,
                },
                FieldDef {
                    index: 16,
                    name: "account_auths".to_string(),
                    ty: TypeRef::FlatMap {
                        key: Box::new(TypeRef::ProtocolObjectId {
                            object_type: "account".to_string(),
                        }),
                        value: Box::new(TypeRef::Uint16),
                        ordering: open_graphene_json_schema::OrderingRule::Unresolved,
                    },
                    source: None,
                    support: None,
                },
                FieldDef {
                    index: 17,
                    name: "key_auths".to_string(),
                    ty: TypeRef::FlatMap {
                        key: Box::new(TypeRef::PublicKey {
                            chain_prefix: None,
                            prefix_ref: Some("chain.publicKeyPrefix".to_string()),
                        }),
                        value: Box::new(TypeRef::Uint16),
                        ordering: open_graphene_json_schema::OrderingRule::Unresolved,
                    },
                    source: None,
                    support: None,
                },
                FieldDef {
                    index: 18,
                    name: "address_auths".to_string(),
                    ty: TypeRef::FlatMap {
                        key: Box::new(TypeRef::Address),
                        value: Box::new(TypeRef::Uint16),
                        ordering: open_graphene_json_schema::OrderingRule::Unresolved,
                    },
                    source: None,
                    support: None,
                },
                FieldDef {
                    index: 19,
                    name: "required_auths".to_string(),
                    ty: TypeRef::Set {
                        inner: Box::new(TypeRef::ProtocolObjectId {
                            object_type: "account".to_string(),
                        }),
                        ordering: open_graphene_json_schema::OrderingRule::Unresolved,
                    },
                    source: None,
                    support: None,
                },
                FieldDef {
                    index: 20,
                    name: "key_auths_set".to_string(),
                    ty: TypeRef::Set {
                        inner: Box::new(TypeRef::PublicKey {
                            chain_prefix: None,
                            prefix_ref: Some("chain.publicKeyPrefix".to_string()),
                        }),
                        ordering: open_graphene_json_schema::OrderingRule::Unresolved,
                    },
                    source: None,
                    support: None,
                },
                FieldDef {
                    index: 21,
                    name: "payload".to_string(),
                    ty: TypeRef::Bytes,
                    source: None,
                    support: None,
                },
                FieldDef {
                    index: 22,
                    name: "digest".to_string(),
                    ty: TypeRef::FixedBytes { bytes: 20 },
                    source: None,
                    support: None,
                },
                FieldDef {
                    index: 23,
                    name: "account".to_string(),
                    ty: TypeRef::ProtocolObjectId {
                        object_type: "account".to_string(),
                    },
                    source: None,
                    support: None,
                },
                FieldDef {
                    index: 24,
                    name: "extensions".to_string(),
                    ty: TypeRef::Set {
                        inner: Box::new(TypeRef::StaticVariantRef {
                            name: "future_extensions".to_string(),
                        }),
                        ordering: OrderingRule::StaticVariantTag,
                    },
                    source: None,
                    support: None,
                },
            ],
            is_virtual: false,
            source: None,
            support: None,
        });
        protocol.operations.push(OperationDef {
            name: "call_order_update_operation".to_string(),
            wire_tag: 3,
            fields: vec![FieldDef {
                index: 0,
                name: "unsupported_signature".to_string(),
                ty: TypeRef::Signature,
                source: None,
                support: None,
            }],
            is_virtual: false,
            source: None,
            support: None,
        });
        protocol.static_variants.push(StaticVariantDef {
            name: "operation".to_string(),
            kind: "static_variant".to_string(),
            json: "tagged_tuple".to_string(),
            fc: "static_variant".to_string(),
            variants: vec![
                open_graphene_json_schema::StaticVariantArmDef {
                    tag: 0,
                    name: "transfer_operation".to_string(),
                    ty: TypeRef::Ref {
                        name: "transfer_operation".to_string(),
                    },
                    support: None,
                },
                open_graphene_json_schema::StaticVariantArmDef {
                    tag: 1,
                    name: "limit_order_create_operation".to_string(),
                    ty: TypeRef::Ref {
                        name: "limit_order_create_operation".to_string(),
                    },
                    support: None,
                },
                open_graphene_json_schema::StaticVariantArmDef {
                    tag: 2,
                    name: "limit_order_cancel_operation".to_string(),
                    ty: TypeRef::Ref {
                        name: "limit_order_cancel_operation".to_string(),
                    },
                    support: None,
                },
                open_graphene_json_schema::StaticVariantArmDef {
                    tag: 3,
                    name: "call_order_update_operation".to_string(),
                    ty: TypeRef::Ref {
                        name: "call_order_update_operation".to_string(),
                    },
                    support: None,
                },
                open_graphene_json_schema::StaticVariantArmDef {
                    tag: 9,
                    name: "custom_supported_operation".to_string(),
                    ty: TypeRef::Ref {
                        name: "custom_supported_operation".to_string(),
                    },
                    support: None,
                },
            ],
            source: None,
            support: None,
        });

        protocol.static_variants.push(StaticVariantDef {
            name: "special_authority".to_string(),
            kind: "static_variant".to_string(),
            json: "tagged_tuple".to_string(),
            fc: "static_variant".to_string(),
            variants: vec![
                open_graphene_json_schema::StaticVariantArmDef {
                    tag: 0,
                    name: "no_special_authority".to_string(),
                    ty: TypeRef::Ref {
                        name: "no_special_authority".to_string(),
                    },
                    support: None,
                },
                open_graphene_json_schema::StaticVariantArmDef {
                    tag: 1,
                    name: "top_holders_special_authority".to_string(),
                    ty: TypeRef::Ref {
                        name: "top_holders_special_authority".to_string(),
                    },
                    support: None,
                },
            ],
            source: None,
            support: None,
        });
        protocol.static_variants.push(StaticVariantDef {
            name: "htlc_hash".to_string(),
            kind: "static_variant".to_string(),
            json: "tagged_tuple".to_string(),
            fc: "static_variant".to_string(),
            variants: vec![
                open_graphene_json_schema::StaticVariantArmDef {
                    tag: 0,
                    name: "htlc_algo_ripemd160".to_string(),
                    ty: TypeRef::FixedBytes { bytes: 20 },
                    support: None,
                },
                open_graphene_json_schema::StaticVariantArmDef {
                    tag: 2,
                    name: "htlc_algo_sha256".to_string(),
                    ty: TypeRef::FixedBytes { bytes: 32 },
                    support: None,
                },
            ],
            source: None,
            support: None,
        });
        protocol.static_variants.push(StaticVariantDef {
            name: "predicate".to_string(),
            kind: "static_variant".to_string(),
            json: "tagged_tuple".to_string(),
            fc: "static_variant".to_string(),
            variants: vec![open_graphene_json_schema::StaticVariantArmDef {
                tag: 2,
                name: "block_id_predicate".to_string(),
                ty: TypeRef::Ref {
                    name: "block_id_predicate".to_string(),
                },
                support: None,
            }],
            source: None,
            support: None,
        });
        protocol.static_variants.push(StaticVariantDef {
            name: "vesting_policy_initializer".to_string(),
            kind: "static_variant".to_string(),
            json: "tagged_tuple".to_string(),
            fc: "static_variant".to_string(),
            variants: vec![open_graphene_json_schema::StaticVariantArmDef {
                tag: 1,
                name: "cdd_vesting_policy_initializer".to_string(),
                ty: TypeRef::Ref {
                    name: "cdd_vesting_policy_initializer".to_string(),
                },
                support: None,
            }],
            source: None,
            support: None,
        });
        protocol.static_variants.push(StaticVariantDef {
            name: "worker_initializer".to_string(),
            kind: "static_variant".to_string(),
            json: "tagged_tuple".to_string(),
            fc: "static_variant".to_string(),
            variants: vec![open_graphene_json_schema::StaticVariantArmDef {
                tag: 1,
                name: "vesting_balance_worker_initializer".to_string(),
                ty: TypeRef::Ref {
                    name: "vesting_balance_worker_initializer".to_string(),
                },
                support: None,
            }],
            source: None,
            support: None,
        });
        protocol.static_variants.push(StaticVariantDef {
            name: "limit_order_auto_action".to_string(),
            kind: "static_variant".to_string(),
            json: "tagged_tuple".to_string(),
            fc: "static_variant".to_string(),
            variants: vec![open_graphene_json_schema::StaticVariantArmDef {
                tag: 0,
                name: "create_take_profit_order_action".to_string(),
                ty: TypeRef::Ref {
                    name: "create_take_profit_order_action".to_string(),
                },
                support: None,
            }],
            source: None,
            support: None,
        });

        let types = render_types(&protocol).expect("render types");
        assert!(types.contains("impl Asset"));
        assert!(
            types.contains(
                "pub fn new(amount: i64, asset_id: crate::generated::ids::AssetId) -> Self"
            )
        );
        assert!(types.contains("Self { amount, asset_id }"));
        assert!(types.contains("impl Price"));
        assert!(types.contains(
            "pub fn new(base: crate::generated::types::Asset, quote: crate::generated::types::Asset) -> Self"
        ));
        assert!(types.contains("Self { base, quote }"));

        let output = render_fc(&protocol).expect("render fc");

        assert!(output.contains("pub use open_graphene_fc::{decode_chain_id_hex, decode_public_key, is_graphene_canonical_compact_signature, parse_protocol_object_id, recover_public_key_from_compact_signature, sha256_bytes, sign_digest_compact_with_wif, verify_compact_signature_public_key, write_bytes, write_fixed_bytes, write_protocol_object_id, write_public_key, write_time_point_sec, write_varint, write_vote_id, FcSerialize, FcSerializeError, Result};"));
        assert!(output.contains("impl FcSerialize for crate::generated::types::Asset"));
        assert!(
            output.contains(
                "impl FcSerialize for crate::generated::static_variants::SpecialAuthority"
            )
        );
        assert!(output.contains("Self::NoSpecialAuthority(value)"));
        assert!(output.contains("Self::TopHoldersSpecialAuthority(value)"));
        assert!(
            output.contains("impl FcSerialize for crate::generated::static_variants::HtlcHash")
        );
        assert!(output.contains("Self::HtlcAlgoRipemd160(value)"));
        assert!(output.contains(
            "write_fixed_bytes(value.as_ref(), 20, \"htlc_hash::htlc_algo_ripemd160\", out)"
        ));
        assert!(output.contains("Self::HtlcAlgoSha256(value)"));
        assert!(output.contains(
            "write_fixed_bytes(value.as_ref(), 32, \"htlc_hash::htlc_algo_sha256\", out)"
        ));
        assert!(
            output.contains("impl FcSerialize for crate::generated::static_variants::Predicate")
        );
        assert!(output.contains("Self::BlockIdPredicate(value)"));
        assert!(output.contains("write_varint(2u64, out);"));
        assert!(output.contains(
            "impl FcSerialize for crate::generated::static_variants::VestingPolicyInitializer"
        ));
        assert!(output.contains("Self::CddVestingPolicyInitializer(value)"));
        assert!(output.contains("write_varint(1u64, out);"));
        assert!(
            output.contains(
                "impl FcSerialize for crate::generated::static_variants::WorkerInitializer"
            )
        );
        assert!(output.contains("Self::VestingBalanceWorkerInitializer(value)"));
        assert!(output.contains(
            "impl FcSerialize for crate::generated::static_variants::LimitOrderAutoAction"
        ));
        assert!(output.contains("Self::CreateTakeProfitOrderAction(value)"));
        assert!(output.contains("self.amount.fc_serialize(out)?;"));
        assert!(output.contains("self.asset_id.fc_serialize(out)?;"));
        assert!(output.contains("impl FcSerialize for crate::generated::types::Signature"));
        assert!(output.contains("write_fixed_bytes(&self.0, 65, \"signature\", out)"));
        assert!(
            output.contains(
                "impl FcSerialize for crate::generated::types::UnsupportedSignatureStruct"
            )
        );
        assert!(output.contains("Self::TransferOperation(value) => { write_varint(0u64, out); value.as_ref().fc_serialize(out) }"));
        assert!(output.contains(
            "impl FcSerialize for crate::generated::operations::LimitOrderCreateOperation"
        ));
        assert!(output.contains("Self::LimitOrderCreateOperation(value) => { write_varint(1u64, out); value.as_ref().fc_serialize(out) }"));
        assert!(output.contains(
            "impl FcSerialize for crate::generated::operations::LimitOrderCancelOperation"
        ));
        assert!(output.contains("Self::LimitOrderCancelOperation(value) => { write_varint(2u64, out); value.as_ref().fc_serialize(out) }"));
        assert!(output.contains(
            "impl FcSerialize for crate::generated::operations::CustomSupportedOperation"
        ));
        assert!(output.contains("self.fee.fc_serialize(out)?;"));
        assert!(output.contains("self.enabled.fc_serialize(out)?;"));
        assert!(output.contains("self.small.fc_serialize(out)?;"));
        assert!(output.contains("self.medium.fc_serialize(out)?;"));
        assert!(output.contains("self.large.fc_serialize(out)?;"));
        assert!(output.contains("self.huge.fc_serialize(out)?;"));
        assert!(output.contains("self.signed.fc_serialize(out)?;"));
        assert!(output.contains("self.label.fc_serialize(out)?;"));
        assert!(output.contains("write_public_key(&self.signing_key, Some(crate::generated::ids::PUBLIC_KEY_PREFIX), out)?;"));
        assert!(output.contains("match &self.optional_signing_key"));
        assert!(output.contains(
            "write_public_key(value, Some(crate::generated::ids::PUBLIC_KEY_PREFIX), out)?;"
        ));
        assert!(output.contains("write_time_point_sec(&self.expires_at, out)?;"));
        assert!(output.contains("match &self.optional_expiration"));
        assert!(output.contains("write_time_point_sec(value, out)?;"));
        assert!(output.contains("write_varint(self.expiration_points.len() as u64, out);"));
        assert!(output.contains("for value in &self.expiration_points"));
        assert!(output.contains("write_vote_id(&self.vote, out)?;"));
        assert!(output.contains("match &self.optional_vote"));
        assert!(output.contains("write_vote_id(&value, out)?;"));
        assert!(output.contains("write_varint(self.votes.len() as u64, out);"));
        assert!(output.contains("for value in &self.votes"));
        assert!(output.contains("write_varint(self.account_auths.len() as u64, out);"));
        assert!(output.contains("for (key, value) in &self.account_auths"));
        assert!(output.contains("parse_protocol_object_id(&key.0, None, None)?;"));
        assert!(output.contains("flat_map keys must be sorted and unique"));
        assert!(output.contains("key.fc_serialize(out)?;"));
        assert!(output.contains("write_varint(self.key_auths.len() as u64, out);"));
        assert!(output.contains("for (key, value) in &self.key_auths"));
        assert!(output.contains("write_public_key(key, Some(crate::generated::ids::PUBLIC_KEY_PREFIX), &mut key_bytes)?;"));
        assert!(output.contains("if !self.address_auths.is_empty()"));
        assert!(output.contains("address flat_map FC serialization is not implemented"));
        assert!(output.contains("write_varint(self.required_auths.len() as u64, out);"));
        assert!(output.contains("for value in &self.required_auths"));
        assert!(output.contains("parse_protocol_object_id(&value.0, None, None)?;"));
        assert!(output.contains("set values must be sorted and unique"));
        assert!(output.contains("write_varint(self.key_auths_set.len() as u64, out);"));
        assert!(output.contains("for value in &self.key_auths_set"));
        assert!(output.contains("write_public_key(value, Some(crate::generated::ids::PUBLIC_KEY_PREFIX), &mut key_bytes)?;"));
        assert!(output.contains("write_bytes(&self.payload, out)?;"));
        assert!(output.contains("write_fixed_bytes(&self.digest, 20, \"fixed_bytes_20\", out)?;"));
        assert!(output.contains("impl FcSerialize for crate::generated::ids::VoteId"));
        assert!(output.contains("write_vote_id(&self.0, out)"));
        assert!(output.contains("self.account.fc_serialize(out)?;"));
        assert!(output.contains("self.extensions.fc_serialize(out)?;"));
        assert!(output.contains("Self::CustomSupportedOperation(value) => { write_varint(9u64, out); value.as_ref().fc_serialize(out) }"));
        assert!(output.contains(
            "impl FcSerialize for crate::generated::operations::CallOrderUpdateOperation"
        ));
        assert!(output.contains("self.unsupported_signature.fc_serialize(out)?;"));
        assert!(output.contains("Self::CallOrderUpdateOperation(value) => { write_varint(3u64, out); value.as_ref().fc_serialize(out) }"));
    }

    #[test]
    fn fc_set_renderer_guards_scalar_and_fixed_bytes_sets_without_sorting() {
        let uint16_set = render_fc_value_serialize_lines(
            "self.restrictions_to_remove",
            &TypeRef::Set {
                inner: Box::new(TypeRef::Uint16),
                ordering: open_graphene_json_schema::OrderingRule::Unresolved,
            },
            "        ",
        )
        .expect("render uint16 set");
        assert!(uint16_set.contains("let mut previous_key: Option<u16> = None;"));
        assert!(uint16_set.contains("previous >= *value"));
        assert!(uint16_set.contains("set values must be sorted and unique"));
        assert!(!uint16_set.contains(".sort"));

        let fixed_bytes_set = render_fc_value_serialize_lines(
            "self.hashes",
            &TypeRef::Set {
                inner: Box::new(TypeRef::FixedBytes { bytes: 32 }),
                ordering: open_graphene_json_schema::OrderingRule::Unresolved,
            },
            "        ",
        )
        .expect("render fixed bytes set");
        assert!(fixed_bytes_set.contains("let mut key_bytes = Vec::new();"));
        assert!(
            fixed_bytes_set
                .contains("write_fixed_bytes(value, 32, \"fixed_bytes_32\", &mut key_bytes)?;")
        );
        assert!(fixed_bytes_set.contains("previous >= &key_bytes"));
        assert!(fixed_bytes_set.contains("out.extend_from_slice(&key_bytes);"));
        assert!(!fixed_bytes_set.contains(".sort"));
    }

    #[test]
    fn fc_argument_type_renderer_emits_tagged_scalar_and_pair_payloads() {
        let mut protocol = minimal_protocol();
        protocol.structs.push(StructDef {
            name: "restriction".to_string(),
            source_name: None,
            kind: StructKind::Struct,
            wire_tag: None,
            fields: vec![FieldDef {
                index: 0,
                name: "member_index".to_string(),
                ty: TypeRef::Uint32,
                source: None,
                support: None,
            }],
            support: None,
        });
        protocol.static_variants.push(StaticVariantDef {
            name: "argument_type".to_string(),
            kind: "static_variant".to_string(),
            json: "tagged_tuple".to_string(),
            fc: "static_variant".to_string(),
            variants: vec![
                open_graphene_json_schema::StaticVariantArmDef {
                    tag: 1,
                    name: "bool".to_string(),
                    ty: TypeRef::Bool,
                    support: None,
                },
                open_graphene_json_schema::StaticVariantArmDef {
                    tag: 41,
                    name: "variant_assert_argument_type".to_string(),
                    ty: TypeRef::Pair {
                        first: Box::new(TypeRef::Int64 {
                            json: None,
                            fc: None,
                        }),
                        second: Box::new(TypeRef::Vector {
                            inner: Box::new(TypeRef::Ref {
                                name: "restriction".to_string(),
                            }),
                        }),
                    },
                    support: None,
                },
            ],
            source: None,
            support: None,
        });

        let mut output = String::new();
        render_fc_argument_type_impl(&mut output, &protocol).expect("render argument_type");

        assert!(
            output.contains("impl FcSerialize for crate::generated::static_variants::ArgumentType")
        );
        assert!(output.contains("Self::Bool(value)"));
        assert!(output.contains("write_varint(1u64, out);"));
        assert!(output.contains("(**value).fc_serialize(out)?;"));
        assert!(output.contains("Self::VariantAssertArgumentType(value)"));
        assert!(output.contains("write_varint(41u64, out);"));
        assert!(output.contains("(**value).0.fc_serialize(out)?;"));
        assert!(output.contains("(**value).1.fc_serialize(out)?;"));
    }

    #[test]
    fn fc_type_support_allows_nested_operation_static_variant() {
        let protocol = minimal_protocol();
        let supported_structs = BTreeSet::new();

        assert!(is_fc_supported_type(
            &protocol,
            &TypeRef::StaticVariantRef {
                name: "operation".to_string(),
            },
            &supported_structs,
        ));
    }

    #[test]
    fn renders_fixed_bytes_json_deserializer_for_hex_strings() {
        let mut protocol = minimal_protocol();
        protocol.structs.push(StructDef {
            name: "dynamic_global_property_object".to_string(),
            source_name: None,
            kind: StructKind::Struct,
            wire_tag: None,
            fields: vec![FieldDef {
                index: 0,
                name: "head_block_id".to_string(),
                ty: TypeRef::FixedBytes { bytes: 20 },
                source: None,
                support: None,
            }],
            support: None,
        });

        let types = render_types(&protocol).expect("render types");

        assert!(
            types
                .contains("pub(crate) fn deserialize_fixed_bytes_20_from_hex_string_or_byte_array")
        );
        assert!(types.contains(
            "#[serde(serialize_with = \"open_graphene_core::serialize_bytes_as_hex\", deserialize_with = \"crate::generated::types::deserialize_fixed_bytes_20_from_hex_string_or_byte_array\")]"
        ));
        assert!(types.contains("pub head_block_id: Vec<u8>,"));
    }

    #[test]
    fn renders_transaction_type_and_fc_impl() {
        let mut protocol = minimal_protocol();
        protocol.chain.chain_id =
            Some("2267f694d96b7ffdcba1a98c63c09e720a18a85ad34954e299c66d5a42234098".to_string());
        protocol.structs.push(StructDef {
            name: "transaction".to_string(),
            source_name: None,
            kind: StructKind::Struct,
            wire_tag: None,
            fields: vec![
                FieldDef {
                    index: 0,
                    name: "ref_block_num".to_string(),
                    ty: TypeRef::Uint16,
                    source: None,
                    support: None,
                },
                FieldDef {
                    index: 1,
                    name: "ref_block_prefix".to_string(),
                    ty: TypeRef::Uint32,
                    source: None,
                    support: None,
                },
                FieldDef {
                    index: 2,
                    name: "expiration".to_string(),
                    ty: TypeRef::TimePointSec,
                    source: None,
                    support: None,
                },
                FieldDef {
                    index: 3,
                    name: "operations".to_string(),
                    ty: TypeRef::Vector {
                        inner: Box::new(TypeRef::StaticVariantRef {
                            name: "operation".to_string(),
                        }),
                    },
                    source: None,
                    support: None,
                },
                FieldDef {
                    index: 4,
                    name: "extensions".to_string(),
                    ty: TypeRef::Set {
                        inner: Box::new(TypeRef::StaticVariantRef {
                            name: "future_extensions".to_string(),
                        }),
                        ordering: OrderingRule::StaticVariantTag,
                    },
                    source: None,
                    support: None,
                },
            ],
            support: None,
        });
        protocol.structs.push(StructDef {
            name: "signed_transaction".to_string(),
            source_name: None,
            kind: StructKind::Struct,
            wire_tag: None,
            fields: vec![
                FieldDef {
                    index: 0,
                    name: "ref_block_num".to_string(),
                    ty: TypeRef::Uint16,
                    source: None,
                    support: None,
                },
                FieldDef {
                    index: 1,
                    name: "ref_block_prefix".to_string(),
                    ty: TypeRef::Uint32,
                    source: None,
                    support: None,
                },
                FieldDef {
                    index: 2,
                    name: "expiration".to_string(),
                    ty: TypeRef::TimePointSec,
                    source: None,
                    support: None,
                },
                FieldDef {
                    index: 3,
                    name: "operations".to_string(),
                    ty: TypeRef::Vector {
                        inner: Box::new(TypeRef::StaticVariantRef {
                            name: "operation".to_string(),
                        }),
                    },
                    source: None,
                    support: None,
                },
                FieldDef {
                    index: 4,
                    name: "extensions".to_string(),
                    ty: TypeRef::Set {
                        inner: Box::new(TypeRef::StaticVariantRef {
                            name: "future_extensions".to_string(),
                        }),
                        ordering: OrderingRule::StaticVariantTag,
                    },
                    source: None,
                    support: None,
                },
                FieldDef {
                    index: 5,
                    name: "signatures".to_string(),
                    ty: TypeRef::Vector {
                        inner: Box::new(TypeRef::Signature),
                    },
                    source: None,
                    support: None,
                },
            ],
            support: None,
        });
        protocol.operations.push(OperationDef {
            name: "transfer_operation".to_string(),
            wire_tag: 0,
            fields: vec![],
            is_virtual: false,
            source: None,
            support: None,
        });
        protocol.static_variants.push(StaticVariantDef {
            name: "operation".to_string(),
            kind: "static_variant".to_string(),
            json: "tagged_tuple".to_string(),
            fc: "static_variant".to_string(),
            variants: vec![open_graphene_json_schema::StaticVariantArmDef {
                tag: 0,
                name: "transfer_operation".to_string(),
                ty: TypeRef::Ref {
                    name: "transfer_operation".to_string(),
                },
                support: None,
            }],
            source: None,
            support: None,
        });
        protocol.static_variants.push(StaticVariantDef {
            name: "future_extensions".to_string(),
            kind: "static_variant".to_string(),
            json: "tagged_tuple".to_string(),
            fc: "static_variant".to_string(),
            variants: vec![open_graphene_json_schema::StaticVariantArmDef {
                tag: 0,
                name: "void_t".to_string(),
                ty: TypeRef::Void,
                support: None,
            }],
            source: None,
            support: None,
        });

        let ids = render_ids(&protocol).expect("render ids");
        assert!(ids.contains(
            "pub const CHAIN_ID_HEX: &str = \"2267f694d96b7ffdcba1a98c63c09e720a18a85ad34954e299c66d5a42234098\";"
        ));

        let types = render_types(&protocol).expect("render types");
        assert!(types.contains("pub struct Signature(\n"));
        assert!(types.contains(
            "deserialize_with = \"open_graphene_core::deserialize_bytes_from_hex_string_or_byte_array\")]\n    pub Vec<u8>,\n);"
        ));
        assert!(types.contains("pub struct Transaction"));
        assert!(types.contains("pub ref_block_num: u16,"));
        assert!(types.contains("pub ref_block_prefix: u32,"));
        assert!(types.contains("pub expiration: String,"));
        assert!(
            types.contains("pub operations: Vec<crate::generated::static_variants::Operation>,")
        );
        assert!(
            types.contains(
                "pub extensions: Vec<crate::generated::static_variants::FutureExtensions>,"
            )
        );
        assert!(types.contains("pub struct SignedTransaction"));
        assert!(types.contains("pub signatures: Vec<crate::generated::types::Signature>,"));

        let fc = render_fc(&protocol).expect("render fc");
        assert!(fc.contains("impl FcSerialize for crate::generated::types::Signature"));
        assert!(fc.contains("write_fixed_bytes(&self.0, 65, \"signature\", out)"));
        assert!(fc.contains("impl FcSerialize for crate::generated::types::Transaction"));
        assert!(fc.contains("self.ref_block_num.fc_serialize(out)?;"));
        assert!(fc.contains("self.ref_block_prefix.fc_serialize(out)?;"));
        assert!(fc.contains("write_time_point_sec(&self.expiration, out)?;"));
        assert!(fc.contains("self.operations.fc_serialize(out)?;"));
        assert!(fc.contains("self.extensions.fc_serialize(out)?;"));
        assert!(fc.contains("Self::TransferOperation(value) => { write_varint(0u64, out); value.as_ref().fc_serialize(out) }"));
        assert!(fc.contains("impl crate::generated::types::Transaction"));
        assert!(fc.contains("pub fn signature_preimage_bytes(&self) -> Result<Vec<u8>>"));
        assert!(fc.contains("decode_chain_id_hex(crate::generated::ids::CHAIN_ID_HEX)?"));
        assert!(fc.contains("self.fc_serialize(&mut out)?;"));
        assert!(fc.contains("pub fn signature_digest_bytes(&self) -> Result<[u8; 32]>"));
        assert!(fc.contains("Ok(sha256_bytes(&self.signature_preimage_bytes()?))"));
        assert!(fc.contains(
            "pub fn sign_with_wif(&self, wif: &str) -> Result<crate::generated::types::Signature>"
        ));
        assert!(fc.contains(
            "sign_digest_compact_with_wif(self.signature_digest_bytes()?, wif)?.to_vec()"
        ));
        assert!(fc.contains(
            "pub fn signed_with_wif(&self, wif: &str) -> Result<crate::generated::types::SignedTransaction>"
        ));
        assert!(fc.contains("signatures: vec![self.sign_with_wif(wif)?]"));
        assert!(fc.contains("impl FcSerialize for crate::generated::types::SignedTransaction"));
        assert!(fc.contains("self.signatures.fc_serialize(out)?;"));
    }

    fn minimal_protocol() -> Protocol {
        Protocol {
            schema_version: 1,
            chain: ChainDef {
                id: "swaplock".to_string(),
                public_key_prefix: "BTS".to_string(),
                chain_id: None,
            },
            structs: vec![],
            enums: vec![],
            static_variants: vec![],
            operations: vec![],
            object_types: vec![open_graphene_json_schema::ObjectTypeDef {
                object_type: "vote".to_string(),
                cpp_alias: "vote_id_type".to_string(),
                object_space: None,
                type_id: None,
                struct_ref: None,
                source: None,
                support: None,
            }],
            rpc_apis: vec![],
            rpc_methods: vec![],
            strict_mode: None,
        }
    }
}
