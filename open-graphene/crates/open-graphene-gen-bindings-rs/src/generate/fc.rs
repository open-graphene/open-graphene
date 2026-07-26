use super::*;

pub(crate) fn render_fc(protocol: &Protocol) -> Result<String> {
    let mut out = generated_header(protocol, "minimal FC serialization for transfer path");
    out.push_str("pub use open_graphene_fc::{decode_chain_id_hex, decode_public_key, is_graphene_canonical_compact_signature, parse_protocol_object_id, recover_public_key_from_compact_signature, sha256_bytes, sign_digest_compact_with_wif, verify_compact_signature_public_key, write_bytes, write_fixed_bytes, write_protocol_object_id, write_public_key, write_time_point_sec, write_varint, write_vote_id, FcSerialize, FcSerializeError, Result};\n\n");

    render_fc_id_impls(&mut out, protocol)?;
    render_fc_signature_impl(&mut out, protocol);
    render_fc_transfer_path_impls(&mut out, protocol)?;
    Ok(out)
}

pub(crate) fn render_fc_signature_impl(out: &mut String, protocol: &Protocol) {
    if !protocol_uses_signature(protocol) {
        return;
    }

    out.push_str("impl FcSerialize for crate::generated::types::Signature {\n");
    out.push_str("    fn fc_serialize(&self, out: &mut Vec<u8>) -> Result<()> {\n");
    out.push_str("        write_fixed_bytes(&self.0, 65, \"signature\", out)\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");
}

pub(crate) fn render_fc_id_impls(out: &mut String, protocol: &Protocol) -> Result<()> {
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

pub(crate) fn render_fc_transfer_path_impls(out: &mut String, protocol: &Protocol) -> Result<()> {
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

    for spec_name in [
        "htlc_hash",
        "predicate",
        "vesting_policy_initializer",
        "worker_initializer",
        "limit_order_auto_action",
        "data_room_subject",
        "data_room_member_ref",
    ] {
        render_fc_tagged_static_variant_impl(out, protocol, spec_name)?;
    }
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

pub(crate) fn render_fc_transaction_helpers(out: &mut String, protocol: &Protocol) {
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
        out.push('\n');
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

pub(crate) fn render_fc_struct_impls(
    out: &mut String,
    protocol: &Protocol,
) -> Result<BTreeSet<String>> {
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
pub(crate) fn extension_struct_names(protocol: &Protocol) -> BTreeSet<String> {
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
pub(crate) fn render_fc_extension_struct_impl(out: &mut String, struct_def: &StructDef) {
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

/// Renders the `FcSerialize` impl for a simple tagged static variant: each arm
/// writes its varint tag followed by the payload. Struct-ref payloads delegate
/// to the payload's own impl; fixed-byte payloads (`htlc_hash`) write the raw
/// digest bytes.
pub(crate) fn render_fc_tagged_static_variant_impl(
    out: &mut String,
    protocol: &Protocol,
    spec_name: &str,
) -> Result<()> {
    let Some(variant) = protocol
        .static_variants
        .iter()
        .find(|variant| variant.name == spec_name)
    else {
        return Ok(());
    };

    out.push_str(&format!(
        "impl FcSerialize for crate::generated::static_variants::{} {{\n",
        rust_type_name(spec_name)
    ));
    out.push_str("    fn fc_serialize(&self, out: &mut Vec<u8>) -> Result<()> {\n");
    out.push_str("        match self {\n");

    let mut arms = variant.variants.clone();
    arms.sort_by_key(|arm| arm.tag);
    for arm in arms {
        let payload = match arm.ty {
            TypeRef::Ref { .. } | TypeRef::Void | TypeRef::ProtocolObjectId { .. } => {
                "value.as_ref().fc_serialize(out)".to_string()
            }
            TypeRef::FixedBytes { bytes } => format!(
                "write_fixed_bytes(value.as_ref(), {bytes}, {}, out)",
                rust_string_literal(&format!("{spec_name}::{}", arm.name))
            ),
            // The generated payload is a String holding a base58 key, not text. Letting it fall
            // through to the generic arm would write length-prefixed UTF-8 instead of the 33
            // compressed bytes the chain reads.
            TypeRef::PublicKey { .. } => {
                let prefix = render_public_key_prefix_expr(&arm.ty)?;
                format!("write_public_key(value.as_ref(), {prefix}, out)")
            }
            _ => {
                return Err(GenBindingsRsError::Render {
                    message: format!(
                        "unsupported {spec_name} variant `{}` payload type for FC rendering",
                        arm.name
                    ),
                });
            }
        };
        let variant_name = rust_variant_name(&arm.name);
        out.push_str(&format!(
            "            Self::{variant_name}(value) => {{\n                write_varint({}u64, out);\n                {payload}\n            }}\n",
            arm.tag
        ));
    }

    out.push_str("        }\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");
    Ok(())
}

pub(crate) fn render_fc_fee_parameters_impl(out: &mut String, protocol: &Protocol) -> Result<()> {
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

pub(crate) fn render_fc_argument_type_impl(out: &mut String, protocol: &Protocol) -> Result<()> {
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
        if !is_fc_supported_type(&arm.ty, &supported_structs) {
            return Err(GenBindingsRsError::Render {
                message: format!(
                    "unsupported argument_type variant `{}` payload type for FC rendering",
                    arm.name
                ),
            });
        }
        let variant_name = rust_variant_name(&arm.name);
        let payload_lines =
            render_fc_value_serialize_lines("(**value)", "value", &arm.ty, "                ")?;
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

pub(crate) fn fc_supported_struct_names(protocol: &Protocol) -> BTreeSet<String> {
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
                .all(|field| is_fc_supported_type(&field.ty, &supported))
            {
                supported.insert(struct_def.name);
                changed = true;
            }
        }
    }

    supported
}

pub(crate) fn render_fc_operation_impls(
    out: &mut String,
    protocol: &Protocol,
    supported_structs: &BTreeSet<String>,
) -> Result<BTreeSet<String>> {
    let mut supported_operations = BTreeSet::new();

    for operation in sorted_operations(&protocol.operations) {
        if operation.name == "transfer_operation" {
            render_fc_transfer_operation_impl(out, &operation)?;
            supported_operations.insert(operation.name);
            continue;
        }

        if !is_fc_supported_operation(&operation, supported_structs) {
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

pub(crate) fn render_fc_field_serialize_line(field: &FieldDef) -> Result<String> {
    let field_name = rust_field_name(&field.name);
    render_fc_value_serialize_lines(
        &format!("self.{field_name}"),
        &format!("&self.{field_name}"),
        &field.ty,
        "        ",
    )
}

/// Renders the statements serializing one value.
///
/// `value_expr` is a place expression of the value's own type (e.g. `self.field`),
/// used for method calls, field access, and iteration borrows. `borrowed_expr` is
/// an expression usable verbatim as a `&T` function argument: `&self.field` for
/// owned places, or just `value` when the surrounding match/loop already binds a
/// reference (borrowing again there would trip clippy's auto-deref/borrow lints).
pub(crate) fn render_fc_value_serialize_lines(
    value_expr: &str,
    borrowed_expr: &str,
    ty: &TypeRef,
    indent: &str,
) -> Result<String> {
    match ty {
        TypeRef::PublicKey { .. } => {
            let prefix = render_public_key_prefix_expr(ty)?;
            Ok(format!(
                "{indent}write_public_key({borrowed_expr}, {prefix}, out)?;\n"
            ))
        }
        TypeRef::TimePointSec => Ok(format!(
            "{indent}write_time_point_sec({borrowed_expr}, out)?;\n"
        )),
        ty if is_vote_id_type(ty) => Ok(format!(
            "{indent}write_vote_id({}, out)?;\n",
            render_vote_id_arg(value_expr, borrowed_expr, ty)?
        )),
        // fc's `unsigned_int`: a base-128 varint, not a fixed-width integer. The generated field
        // is a plain `u64`, so without this arm it would fall through to `fc_serialize` and be
        // written as eight little-endian bytes.
        TypeRef::UnsignedVarint => Ok(format!("{indent}write_varint({value_expr}, out);\n")),
        TypeRef::Bytes => Ok(format!("{indent}write_bytes({borrowed_expr}, out)?;\n")),
        TypeRef::FixedBytes { bytes } => Ok(format!(
            "{indent}write_fixed_bytes({borrowed_expr}, {bytes}, {}, out)?;\n",
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
            render_vote_id_ref_arg("value", inner)?
        )),
        TypeRef::Optional { inner } if matches!(inner.as_ref(), TypeRef::FlatMap { key, value, .. } if is_fc_supported_flat_map(key, value)) =>
        {
            let TypeRef::FlatMap { key, value, .. } = inner.as_ref() else {
                unreachable!("guard checked flat_map inner")
            };
            // `value` is already a reference inside the `Some(value)` arm.
            let inner_lines =
                render_fc_flat_map_serialize_lines("value", "value", key, value, indent)?;
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
                render_vote_id_ref_arg("value", inner)?
            ))
        }
        TypeRef::Pair { first, second } => {
            let first_lines = render_fc_value_serialize_lines(
                &format!("{value_expr}.0"),
                &format!("&{value_expr}.0"),
                first,
                indent,
            )?;
            let second_lines = render_fc_value_serialize_lines(
                &format!("{value_expr}.1"),
                &format!("&{value_expr}.1"),
                second,
                indent,
            )?;
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
            render_fc_flat_map_serialize_lines(
                value_expr,
                &format!("&{value_expr}"),
                key,
                value,
                indent,
            )
        }
        _ => Ok(format!("{indent}{value_expr}.fc_serialize(out)?;\n")),
    }
}

pub(crate) fn is_vote_id_type(ty: &TypeRef) -> bool {
    match ty {
        TypeRef::VoteId => true,
        TypeRef::ProtocolObjectId { object_type } => object_type == "vote",
        _ => false,
    }
}

pub(crate) fn render_fc_set_serialize_lines(
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

pub(crate) fn render_fc_ordered_copy_set_serialize_lines(
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

pub(crate) fn render_fc_static_variant_set_serialize_lines(
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

pub(crate) fn render_fc_flat_map_serialize_lines(
    value_expr: &str,
    iter_expr: &str,
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
        TypeRef::Uint32 => Ok(format!(
            "{indent}write_varint({value_expr}.len() as u64, out);\n\
             {indent}let mut previous_key: Option<u32> = None;\n\
             {indent}for (key, value) in {iter_expr} {{\n\
             {indent}    if previous_key.is_some_and(|previous| previous >= *key) {{\n\
             {indent}        return Err(FcSerializeError::UnsupportedValue {{ type_name: \"FlatMap\", reason: \"flat_map keys must be sorted and unique\" }});\n\
             {indent}    }}\n\
             {indent}    previous_key = Some(*key);\n\
             {indent}    key.fc_serialize(out)?;\n\
             {indent}    value.fc_serialize(out)?;\n\
             {indent}}}\n"
        )),
        TypeRef::ProtocolObjectId { .. } => Ok(format!(
            "{indent}write_varint({value_expr}.len() as u64, out);\n\
             {indent}let mut previous_key: Option<u64> = None;\n\
             {indent}for (key, value) in {iter_expr} {{\n\
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
                 {indent}for (key, value) in {iter_expr} {{\n\
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

pub(crate) fn is_fc_supported_set(inner: &TypeRef) -> bool {
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

pub(crate) fn is_fee_parameters_type(ty: &TypeRef) -> bool {
    matches!(ty, TypeRef::StaticVariantRef { name } if name == "fee_parameters")
}

pub(crate) fn is_future_extensions_type(ty: &TypeRef) -> bool {
    matches!(ty, TypeRef::StaticVariantRef { name } if name == "future_extensions")
}

pub(crate) fn is_fc_supported_flat_map(key: &TypeRef, value: &TypeRef) -> bool {
    if matches!(key, TypeRef::Address) {
        return matches!(value, TypeRef::Uint16);
    }

    if matches!(key, TypeRef::Uint32) {
        return matches!(value, TypeRef::String);
    }

    matches!(
        key,
        TypeRef::ProtocolObjectId { .. } | TypeRef::PublicKey { .. }
    ) && (matches!(
        value,
        TypeRef::Uint16 | TypeRef::Int64 { json: None, .. } | TypeRef::String
    ) || matches!(value, TypeRef::Ref { name } if name == "price"))
}

pub(crate) fn render_vote_id_arg(
    value_expr: &str,
    borrowed_expr: &str,
    ty: &TypeRef,
) -> Result<String> {
    match ty {
        TypeRef::VoteId => Ok(borrowed_expr.to_string()),
        TypeRef::ProtocolObjectId { object_type } if object_type == "vote" => {
            Ok(format!("&{value_expr}.0"))
        }
        _ => Err(GenBindingsRsError::Render {
            message: "internal error: expected vote id type".to_string(),
        }),
    }
}

/// Like [`render_vote_id_arg`], but for loop/match bindings where the
/// variable is already a reference and an extra borrow would be redundant.
pub(crate) fn render_vote_id_ref_arg(value_expr: &str, ty: &TypeRef) -> Result<String> {
    match ty {
        TypeRef::VoteId => Ok(value_expr.to_string()),
        TypeRef::ProtocolObjectId { object_type } if object_type == "vote" => {
            Ok(format!("&{value_expr}.0"))
        }
        _ => Err(GenBindingsRsError::Render {
            message: "internal error: expected vote id type".to_string(),
        }),
    }
}

pub(crate) fn render_public_key_prefix_expr(ty: &TypeRef) -> Result<String> {
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

pub(crate) fn render_fc_transfer_operation_impl(
    out: &mut String,
    operation: &OperationDef,
) -> Result<()> {
    // This impl hand-encodes the transfer wire layout (with memo kept
    // fail-closed pending verified test vectors). It must never compile
    // against a fork whose transfer_operation has a different shape:
    // verify the spec matches the assumed field sequence exactly.
    const EXPECTED_FIELDS: [&str; 6] = ["fee", "from", "to", "amount", "memo", "extensions"];
    let mut fields = operation.fields.clone();
    fields.sort_by_key(|field| field.index);
    let actual: Vec<&str> = fields.iter().map(|field| field.name.as_str()).collect();
    if actual != EXPECTED_FIELDS {
        return Err(GenBindingsRsError::Render {
            message: format!(
                "transfer_operation FC template assumes fields {EXPECTED_FIELDS:?} but the spec declares {actual:?}; update the template before generating signing code"
            ),
        });
    }
    let memo_is_optional = fields
        .iter()
        .find(|field| field.name == "memo")
        .is_some_and(|field| matches!(field.ty, TypeRef::Optional { .. }));
    if !memo_is_optional {
        return Err(GenBindingsRsError::Render {
            message: "transfer_operation FC template assumes an optional memo field".to_string(),
        });
    }

    out.push_str("impl FcSerialize for crate::generated::operations::TransferOperation {\n");
    out.push_str("    fn fc_serialize(&self, out: &mut Vec<u8>) -> Result<()> {\n");
    out.push_str("        self.fee.fc_serialize(out)?;\n");
    out.push_str("        self.from.fc_serialize(out)?;\n");
    out.push_str("        self.to.fc_serialize(out)?;\n");
    out.push_str("        self.amount.fc_serialize(out)?;\n");
    // optional<memo_data>: 0x00 when absent, 0x01 followed by the memo when present.
    // MemoData has a full FcSerialize impl of its own (see the struct templates), so
    // refusing to serialise a present memo would only force callers to hand-patch the
    // generated file — which is exactly how this output drifted from the generator before.
    out.push_str("        match &self.memo {\n");
    out.push_str("            Some(memo) => {\n");
    out.push_str("                out.push(1);\n");
    out.push_str("                memo.fc_serialize(out)?;\n");
    out.push_str("            }\n");
    out.push_str("            None => {\n");
    out.push_str("                out.push(0);\n");
    out.push_str("            }\n");
    out.push_str("        }\n");
    out.push_str("        self.extensions.fc_serialize(out)?;\n");
    out.push_str("        Ok(())\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");
    Ok(())
}

pub(crate) fn is_fc_supported_operation(
    operation: &OperationDef,
    supported_structs: &BTreeSet<String>,
) -> bool {
    operation
        .fields
        .iter()
        .all(|field| is_fc_supported_type(&field.ty, supported_structs))
}

pub(crate) fn is_fc_supported_type(ty: &TypeRef, supported_structs: &BTreeSet<String>) -> bool {
    match ty {
        TypeRef::Void
        | TypeRef::Bool
        | TypeRef::Uint8
        | TypeRef::Uint16
        | TypeRef::Uint32
        | TypeRef::UnsignedVarint
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
                || name == "data_room_subject"
                || name == "data_room_member_ref"
        }
        TypeRef::Optional { inner } | TypeRef::Vector { inner } => {
            is_fc_supported_type(inner, supported_structs)
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
            is_fc_supported_type(first, supported_structs)
                && is_fc_supported_type(second, supported_structs)
        }
        TypeRef::Set { .. }
        | TypeRef::Map { .. }
        | TypeRef::FlatMap { .. }
        | TypeRef::Int64 { .. }
        | TypeRef::Uint64 { .. }
        | TypeRef::Uint128 { .. }
        | TypeRef::CallbackHandle
        | TypeRef::FixedHex { .. }
        | TypeRef::TimePoint
        | TypeRef::Address
        | TypeRef::ProtocolObjectUnion { .. }
        | TypeRef::AnyJson { .. }
        | TypeRef::Unsupported { .. } => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::test_support::{minimal_protocol, transfer_operation_fields};
    use open_graphene_json_schema::types::OrderingRule;

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
            fields: transfer_operation_fields(),
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

        let types = render_types(&protocol, &detect_schema_no_recursion_cuts(&protocol))
            .expect("render types");
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
        assert!(output.contains("write_vote_id(value, out)?;"));
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
            "&self.restrictions_to_remove",
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
            "&self.hashes",
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
        let supported_structs = BTreeSet::new();

        assert!(is_fc_supported_type(
            &TypeRef::StaticVariantRef {
                name: "operation".to_string(),
            },
            &supported_structs,
        ));
    }

    #[test]
    fn renders_transaction_type_and_fc_impl() {
        let mut protocol = minimal_protocol();
        protocol.chain.chain_id =
            Some("9118895266b1e75e8c30b0e8433cf6cfab32ac59b6b1e1107fe2f58affc216f9".to_string());
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
            fields: transfer_operation_fields(),
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
            "pub const CHAIN_ID_HEX: &str = \"9118895266b1e75e8c30b0e8433cf6cfab32ac59b6b1e1107fe2f58affc216f9\";"
        ));

        let types = render_types(&protocol, &detect_schema_no_recursion_cuts(&protocol))
            .expect("render types");
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
}
