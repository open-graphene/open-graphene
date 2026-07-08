use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use open_graphene_json_schema::{
    EnumDef, FieldDef, JsonShape, OperationDef, Protocol, RpcMethodDef, StaticVariantDef,
    StructDef, StructKind, TypeRef,
};

use crate::error::{GenBindingsRsError, Result};

mod cycles;
mod fc;
mod ids;
mod naming;
mod rpc;
mod static_variants;
#[cfg(test)]
mod test_support;
mod types;

use cycles::*;
use fc::*;
use ids::*;
use naming::*;
use rpc::*;
use static_variants::*;
use types::*;

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

    let no_recursion_cuts = detect_schema_no_recursion_cuts(&protocol);
    let files = [
        ("mod.rs", render_mod(&protocol)?),
        ("ids.rs", render_ids(&protocol)?),
        ("types.rs", render_types(&protocol, &no_recursion_cuts)?),
        (
            "operations.rs",
            render_operations(&protocol, &no_recursion_cuts)?,
        ),
        (
            "static_variants.rs",
            render_static_variants(&protocol, &no_recursion_cuts)?,
        ),
        ("fc.rs", render_fc(&protocol)?),
        ("rpc.rs", render_rpc(&protocol)?),
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
    out.push_str("pub mod rpc;\n");
    out.push_str("pub mod static_variants;\n");
    out.push_str("pub mod types;\n\n");
    out.push_str("pub use fc::*;\n");
    out.push_str("pub use ids::*;\n");
    out.push_str("pub use operations::*;\n");
    out.push_str("pub use static_variants::*;\n");
    out.push_str("pub use types::*;\n");
    Ok(out)
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

fn generated_header(protocol: &Protocol, module: &str) -> String {
    let allows = match module {
        // TODO: drop these once the emission templates produce clippy-clean
        // code (planned alongside the generate.rs split): auto-deref/borrow
        // patterns in fc serialization, vec building and Copy-aware borrows
        // in rpc params, and boxed ProtocolObject variants.
        "minimal FC serialization for transfer path" => {
            "#![allow(clippy::explicit_auto_deref, clippy::borrow_deref_ref)]\n\n"
        }
        "typed RPC call surface" => {
            "#![allow(clippy::vec_init_then_push, clippy::needless_borrows_for_generic_args)]\n\
             #![allow(clippy::large_enum_variant)]\n\n"
        }
        _ => "",
    };
    format!(
        "// Generated by open-graphene-gen-bindings-rs.\n\
         // Chain: {} | schema version: {} | module: {module}.\n\
         // Do not edit by hand.\n\n{allows}",
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
