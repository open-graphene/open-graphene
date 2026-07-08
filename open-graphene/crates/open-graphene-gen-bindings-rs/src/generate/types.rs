use super::*;

pub(crate) fn render_types(
    protocol: &Protocol,
    no_recursion_cuts: &SchemaNoRecursionCuts,
) -> Result<String> {
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
        render_struct(&mut out, protocol, &struct_def, no_recursion_cuts)?;
    }

    Ok(out)
}

pub(crate) fn render_operations(
    protocol: &Protocol,
    no_recursion_cuts: &SchemaNoRecursionCuts,
) -> Result<String> {
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
        render_operation_struct(&mut out, protocol, &operation, no_recursion_cuts)?;
    }

    Ok(out)
}

pub(crate) fn render_signature_type(out: &mut String, protocol: &Protocol) {
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

pub(crate) fn render_enum(out: &mut String, protocol: &Protocol, enum_def: &EnumDef) -> Result<()> {
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

pub(crate) fn render_struct(
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
    render_fields(
        out,
        protocol,
        &struct_name,
        &struct_def.fields,
        no_recursion_cuts,
    )?;
    out.push_str("}\n\n");
    render_asset_constructor(out, protocol, struct_def)?;
    render_price_constructor(out, protocol, struct_def)?;
    Ok(())
}

pub(crate) fn render_asset_constructor(
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

pub(crate) fn render_price_constructor(
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

pub(crate) fn render_operation_struct(
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
    render_fields(
        out,
        protocol,
        &struct_name,
        &operation.fields,
        no_recursion_cuts,
    )?;
    out.push_str("}\n\n");
    Ok(())
}

pub(crate) fn render_fields(
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

pub(crate) fn render_type_ref(protocol: &Protocol, ty: &TypeRef) -> Result<String> {
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

pub(crate) fn protocol_uses_fixed_bytes(protocol: &Protocol) -> bool {
    !fixed_byte_lengths(protocol).is_empty()
}

pub(crate) fn fixed_byte_lengths(protocol: &Protocol) -> BTreeSet<usize> {
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

pub(crate) fn collect_fixed_byte_lengths_from_type(ty: &TypeRef, lengths: &mut BTreeSet<usize>) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::test_support::minimal_protocol;

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

        let types = render_types(&protocol, &detect_schema_no_recursion_cuts(&protocol))
            .expect("render types");

        assert!(
            types
                .contains("pub(crate) fn deserialize_fixed_bytes_20_from_hex_string_or_byte_array")
        );
        assert!(types.contains(
            "#[serde(serialize_with = \"open_graphene_core::serialize_bytes_as_hex\", deserialize_with = \"crate::generated::types::deserialize_fixed_bytes_20_from_hex_string_or_byte_array\")]"
        ));
        assert!(types.contains("pub head_block_id: Vec<u8>,"));
    }
}
