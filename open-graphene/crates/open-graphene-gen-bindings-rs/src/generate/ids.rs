use super::*;

pub(crate) fn render_ids(protocol: &Protocol) -> Result<String> {
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
        if let Some(object_type) = object_type
            && let Some(space) = object_type.object_space
        {
            out.push_str(&format!("impl {id_name} {{\n"));
            out.push_str(&format!("    pub const SPACE_ID: u32 = {space};\n"));
            if let Some(type_id) = object_type.type_id {
                out.push_str(&format!("    pub const TYPE_ID: u32 = {type_id};\n"));
            }
            out.push_str("}\n");
        }
        out.push('\n');
    }

    Ok(out)
}

pub(crate) fn render_string_id_conversions(out: &mut String, id_name: &str) {
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

pub(crate) fn collect_protocol_object_id_names(protocol: &Protocol) -> BTreeSet<String> {
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

pub(crate) fn collect_protocol_object_id_names_from_type(
    ty: &TypeRef,
    names: &mut BTreeSet<String>,
) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::test_support::minimal_protocol;

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
}
