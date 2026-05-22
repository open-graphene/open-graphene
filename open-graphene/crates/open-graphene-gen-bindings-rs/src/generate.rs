use std::collections::BTreeSet;
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
        fs::write(&path, contents)
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
        "#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]\n",
    );
    out.push_str("#[serde(transparent)]\n");
    out.push_str("pub struct ObjectId(pub String);\n\n");

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
        out.push_str("#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]\n");
        out.push_str("#[serde(transparent)]\n");
        out.push_str(&format!("pub struct {id_name}(pub String);\n"));
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

fn render_types(protocol: &Protocol) -> Result<String> {
    let mut out = generated_header(protocol, "raw structs and enums");
    out.push_str("use serde::{Deserialize, Serialize};\n\n");

    let mut emitted = BTreeSet::new();
    for enum_def in sorted_enums(&protocol.enums) {
        let name = rust_type_name(&enum_def.name);
        ensure_unique(&mut emitted, &name, "type")?;
        render_enum(&mut out, &enum_def)?;
    }

    for struct_def in sorted_structs(&protocol.structs) {
        if struct_def.kind == StructKind::Operation || is_operation_ref(protocol, &struct_def.name)
        {
            continue;
        }
        let name = rust_type_name(&struct_def.name);
        ensure_unique(&mut emitted, &name, "type")?;
        render_struct(&mut out, protocol, &struct_def)?;
    }

    Ok(out)
}

fn render_operations(protocol: &Protocol) -> Result<String> {
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
        render_operation_struct(&mut out, protocol, &operation)?;
    }

    Ok(out)
}

fn render_static_variants(protocol: &Protocol) -> Result<String> {
    let mut out = generated_header(protocol, "static variant enums");
    out.push_str("// Static variants use Graphene JSON wire format: [tag, value].\n\n");

    let mut emitted = BTreeSet::new();
    for variant in sorted_static_variants(&protocol.static_variants) {
        let name = rust_type_name(&variant.name);
        ensure_unique(&mut emitted, &name, "static variant")?;
        render_static_variant(&mut out, protocol, &variant)?;
    }

    Ok(out)
}

fn render_enum(out: &mut String, enum_def: &EnumDef) -> Result<()> {
    let enum_name = rust_type_name(&enum_def.name);
    out.push_str(&format!(
        "/// Raw enum `{}`. Numeric wire serde is not implemented yet.\n",
        enum_def.name
    ));
    out.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]\n");
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

fn render_struct(out: &mut String, protocol: &Protocol, struct_def: &StructDef) -> Result<()> {
    let struct_name = rust_type_name(&struct_def.name);
    out.push_str(&format!("/// Raw protocol struct `{}`.\n", struct_def.name));
    out.push_str("#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]\n");
    out.push_str(&format!("pub struct {struct_name} {{\n"));
    render_fields(out, protocol, &struct_def.fields)?;
    out.push_str("}\n\n");
    Ok(())
}

fn render_operation_struct(
    out: &mut String,
    protocol: &Protocol,
    operation: &OperationDef,
) -> Result<()> {
    let struct_name = rust_type_name(&operation.name);
    out.push_str(&format!(
        "/// Protocol operation `{}` with wire tag {}.\n",
        operation.name, operation.wire_tag
    ));
    out.push_str("#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]\n");
    out.push_str(&format!("pub struct {struct_name} {{\n"));
    render_fields(out, protocol, &operation.fields)?;
    out.push_str("}\n\n");
    Ok(())
}

fn render_fields(out: &mut String, protocol: &Protocol, fields: &[FieldDef]) -> Result<()> {
    let mut fields = fields.to_vec();
    fields.sort_by_key(|field| field.index);
    let mut emitted = BTreeSet::new();

    for field in fields {
        let field_name = rust_field_name(&field.name);
        ensure_unique(&mut emitted, &field_name, "field")?;
        let ty = render_type_ref(protocol, &field.ty)?;
        out.push_str(&serde_rename_attr("    ", &field.name, &field_name));
        out.push_str(&format!("    pub {field_name}: {ty},\n"));
    }

    Ok(())
}

fn render_fc(protocol: &Protocol) -> Result<String> {
    let mut out = generated_header(protocol, "minimal FC serialization for transfer path");
    out.push_str("pub use open_graphene_fc::{write_protocol_object_id, write_varint, FcSerialize, FcSerializeError, Result};\n\n");

    render_fc_id_impls(&mut out, protocol)?;
    render_fc_transfer_path_impls(&mut out, protocol)?;
    Ok(out)
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
        out.push_str(&format!(
            "        write_protocol_object_id(&self.0, {expected_space}, {expected_type}, out)\n"
        ));
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
    if has_type(protocol, "asset") {
        out.push_str("impl FcSerialize for crate::generated::types::Asset {\n");
        out.push_str("    fn fc_serialize(&self, out: &mut Vec<u8>) -> Result<()> {\n");
        out.push_str("        self.amount.fc_serialize(out)?;\n");
        out.push_str("        self.asset_id.fc_serialize(out)?;\n");
        out.push_str("        Ok(())\n");
        out.push_str("    }\n");
        out.push_str("}\n\n");
    }

    if protocol
        .static_variants
        .iter()
        .any(|variant| variant.name == "future_extensions")
    {
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

    let transfer_operation = protocol
        .operations
        .iter()
        .find(|operation| operation.name == "transfer_operation");
    if transfer_operation.is_some() {
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

    let limit_order_create_operation = protocol
        .operations
        .iter()
        .find(|operation| operation.name == "limit_order_create_operation");
    if limit_order_create_operation.is_some() {
        out.push_str("impl FcSerialize for crate::generated::operations::LimitOrderCreateOperation {\n");
        out.push_str("    fn fc_serialize(&self, out: &mut Vec<u8>) -> Result<()> {\n");
        out.push_str("        self.fee.fc_serialize(out)?;\n");
        out.push_str("        self.seller.fc_serialize(out)?;\n");
        out.push_str("        self.amount_to_sell.fc_serialize(out)?;\n");
        out.push_str("        self.min_to_receive.fc_serialize(out)?;\n");
        out.push_str("        self.fill_or_kill.fc_serialize(out)?;\n");
        out.push_str("        self.extensions.fc_serialize(out)?;\n");
        out.push_str("        Ok(())\n");
        out.push_str("    }\n");
        out.push_str("}\n\n");
    }

    let limit_order_cancel_operation = protocol
        .operations
        .iter()
        .find(|operation| operation.name == "limit_order_cancel_operation");
    if limit_order_cancel_operation.is_some() {
        out.push_str("impl FcSerialize for crate::generated::operations::LimitOrderCancelOperation {\n");
        out.push_str("    fn fc_serialize(&self, out: &mut Vec<u8>) -> Result<()> {\n");
        out.push_str("        self.fee.fc_serialize(out)?;\n");
        out.push_str("        self.fee_paying_account.fc_serialize(out)?;\n");
        out.push_str("        self.order.fc_serialize(out)?;\n");
        out.push_str("        self.extensions.fc_serialize(out)?;\n");
        out.push_str("        Ok(())\n");
        out.push_str("    }\n");
        out.push_str("}\n\n");
    }

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
            if matches!(
                arm.name.as_str(),
                "transfer_operation"
                    | "limit_order_create_operation"
                    | "limit_order_cancel_operation"
            ) {
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

    Ok(())
}

fn has_type(protocol: &Protocol, name: &str) -> bool {
    protocol
        .structs
        .iter()
        .any(|struct_def| struct_def.name == name)
}

fn render_static_variant(
    out: &mut String,
    protocol: &Protocol,
    variant: &StaticVariantDef,
) -> Result<()> {
    let enum_name = rust_type_name(&variant.name);
    out.push_str(&format!(
        "/// Static variant `{}` serialized as Graphene `[tag, value]`.\n",
        variant.name
    ));
    out.push_str("#[derive(Debug, Clone, PartialEq)]\n");
    out.push_str(&format!("pub enum {enum_name} {{\n"));

    let mut variants = variant.variants.clone();
    variants.sort_by_key(|arm| arm.tag);
    let mut emitted = BTreeSet::new();
    let mut rendered_arms = Vec::new();
    for arm in variants {
        let variant_name = rust_variant_name(&arm.name);
        ensure_unique(&mut emitted, &variant_name, "static variant arm")?;
        let ty = render_type_ref(protocol, &arm.ty)?;
        out.push_str(&format!("    {variant_name}(Box<{ty}>),\n"));
        rendered_arms.push((arm.tag, variant_name, ty));
    }

    out.push_str("}\n\n");
    render_static_variant_serialize_impl(out, &enum_name, &rendered_arms);
    render_static_variant_deserialize_impl(out, &enum_name, &rendered_arms);
    Ok(())
}

fn render_static_variant_serialize_impl(
    out: &mut String,
    enum_name: &str,
    arms: &[(u32, String, String)],
) {
    out.push_str(&format!("impl serde::Serialize for {enum_name} {{\n"));
    out.push_str("    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>\n");
    out.push_str("    where\n");
    out.push_str("        S: serde::Serializer,\n");
    out.push_str("    {\n");
    out.push_str("        use serde::ser::SerializeSeq;\n");
    out.push_str("        let mut seq = serializer.serialize_seq(Some(2))?;\n");
    out.push_str("        match self {\n");
    for (tag, variant_name, _) in arms {
        out.push_str(&format!(
            "            Self::{variant_name}(value) => {{\n                seq.serialize_element(&{tag}u32)?;\n                seq.serialize_element(value.as_ref())?;\n            }}\n"
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
    arms: &[(u32, String, String)],
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
    for (tag, variant_name, ty) in arms {
        out.push_str(&format!(
            "            {tag} => serde_json::from_value::<{ty}>(payload)\n                .map(|value| Self::{variant_name}(Box::new(value)))\n                .map_err(serde::de::Error::custom),\n"
        ));
    }
    out.push_str(&format!(
        "            other => Err(serde::de::Error::custom(format!(\"unknown static variant {enum_name} tag {{other}}\"))),\n"
    ));
    out.push_str("        }\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");
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
        TypeRef::UnsignedVarint | TypeRef::CallbackHandle => "u64".to_string(),
        TypeRef::String
        | TypeRef::FixedHex { .. }
        | TypeRef::TimePointSec
        | TypeRef::TimePoint
        | TypeRef::PublicKey { .. }
        | TypeRef::Address
        | TypeRef::Signature
        | TypeRef::VoteId => "String".to_string(),
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
        render_fields(&mut out, &protocol, &fields).expect("render fields");

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
        render_static_variant(&mut out, &protocol, &variant).expect("render static variant");

        assert!(out.contains("impl serde::Serialize for Operation"));
        assert!(out.contains("seq.serialize_element(&0u32)?;"));
        assert!(out.contains("impl<'de> serde::Deserialize<'de> for Operation"));
        assert!(out.contains("0 => serde_json::from_value::<String>(payload)"));
    }

    #[test]
    fn fc_renderer_emits_transfer_path_impls_and_explicit_unsupported_variants() {
        let mut protocol = minimal_protocol();
        protocol.structs.push(StructDef {
            name: "asset".to_string(),
            source_name: None,
            kind: StructKind::Struct,
            wire_tag: None,
            fields: vec![],
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
            name: "call_order_update_operation".to_string(),
            wire_tag: 3,
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
            ],
            source: None,
            support: None,
        });

        let output = render_fc(&protocol).expect("render fc");

        assert!(output.contains("pub use open_graphene_fc::{write_protocol_object_id, write_varint, FcSerialize, FcSerializeError, Result};"));
        assert!(output.contains("impl FcSerialize for crate::generated::types::Asset"));
        assert!(output.contains("Self::TransferOperation(value) => { write_varint(0u64, out); value.as_ref().fc_serialize(out) }"));
        assert!(output.contains("impl FcSerialize for crate::generated::operations::LimitOrderCreateOperation"));
        assert!(output.contains("Self::LimitOrderCreateOperation(value) => { write_varint(1u64, out); value.as_ref().fc_serialize(out) }"));
        assert!(output.contains("impl FcSerialize for crate::generated::operations::LimitOrderCancelOperation"));
        assert!(output.contains("Self::LimitOrderCancelOperation(value) => { write_varint(2u64, out); value.as_ref().fc_serialize(out) }"));
        assert!(output.contains(
            "Self::CallOrderUpdateOperation(_) => Err(FcSerializeError::UnsupportedVariant"
        ));
    }

    fn minimal_protocol() -> Protocol {
        Protocol {
            schema_version: 1,
            chain: ChainDef {
                id: "swaplock".to_string(),
                public_key_prefix: "BTS".to_string(),
            },
            structs: vec![],
            enums: vec![],
            static_variants: vec![],
            operations: vec![],
            object_types: vec![],
            rpc_apis: vec![],
            rpc_methods: vec![],
            strict_mode: None,
        }
    }
}
