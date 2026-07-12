use super::*;

pub(crate) fn render_rpc(protocol: &Protocol) -> Result<String> {
    let mut out = generated_header(protocol, "typed RPC call surface");
    out.push_str(
        "//! Typed positional parameters and response decoding for the chain's RPC methods.\n\
         //!\n\
         //! Transport stays outside this module: build a [`Params`] value, send it with the\n\
         //! session's `*_call(METHOD, params.to_params_value()?)`, then decode the response\n\
         //! with `parse_returns`. Byte-typed values travel as hex strings on the JSON wire,\n\
         //! and types the spec cannot fully model fall back to `serde_json::Value`.\n\n",
    );

    if protocol.rpc_methods.iter().any(|method| {
        method
            .returns
            .as_ref()
            .is_some_and(contains_protocol_object_union)
    }) {
        render_protocol_object_union(&mut out, protocol)?;
    }

    let mut apis: BTreeMap<String, Vec<&RpcMethodDef>> = BTreeMap::new();
    for method in &protocol.rpc_methods {
        let api_name = method
            .api_name
            .clone()
            .unwrap_or_else(|| method.api_class.clone());
        apis.entry(api_name).or_default().push(method);
    }

    for (api_name, mut methods) in apis {
        methods.sort_by(|a, b| a.name.cmp(&b.name));
        out.push_str(&format!(
            "/// RPC methods on the `{api_name}` API.\npub mod {} {{\n",
            rust_field_name(&api_name)
        ));
        let mut emitted = BTreeSet::new();
        for method in methods {
            render_rpc_method(&mut out, protocol, &api_name, method, &mut emitted)?;
        }
        out.push_str("}\n\n");
    }
    Ok(out)
}

pub(crate) fn contains_protocol_object_union(ty: &TypeRef) -> bool {
    match ty {
        TypeRef::ProtocolObjectUnion { .. } => true,
        TypeRef::Optional { inner } | TypeRef::Vector { inner } | TypeRef::Set { inner, .. } => {
            contains_protocol_object_union(inner)
        }
        TypeRef::Map { key, value, .. } | TypeRef::FlatMap { key, value, .. } => {
            contains_protocol_object_union(key) || contains_protocol_object_union(value)
        }
        TypeRef::Pair { first, second } => {
            contains_protocol_object_union(first) || contains_protocol_object_union(second)
        }
        _ => false,
    }
}

pub(crate) fn render_protocol_object_union(out: &mut String, protocol: &Protocol) -> Result<()> {
    let mut variants = protocol
        .object_types
        .iter()
        .filter_map(|object_type| {
            Some((
                object_type.object_space?,
                object_type.type_id?,
                rust_type_name(&object_type.object_type),
                rust_type_name(object_type.struct_ref.as_ref()?),
            ))
        })
        .collect::<Vec<_>>();
    variants.sort_by_key(|(space, type_id, _, _)| (*space, *type_id));

    out.push_str("/// Any modeled protocol object returned by generic object RPCs.\n");
    out.push_str("///\n");
    out.push_str("/// Deserialization dispatches by the object's `id` (`space.type.instance`) so objects are\n");
    out.push_str("/// not accidentally matched by shape. Object ids whose type is not modeled yet are kept as\n");
    out.push_str("/// raw JSON in [`ProtocolObject::Unknown`].\n");
    out.push_str("///\n");
    out.push_str(
        "/// Payloads are boxed so the enum stays small regardless of the largest object type.\n",
    );
    out.push_str("#[derive(Debug, Clone, PartialEq)]\n");
    out.push_str("pub enum ProtocolObject {\n");
    for (_, _, variant_name, struct_name) in &variants {
        out.push_str(&format!(
            "    {variant_name}(Box<crate::generated::types::{struct_name}>),\n"
        ));
    }
    out.push_str("    Unknown(serde_json::Value),\n");
    out.push_str("}\n\n");

    out.push_str("impl serde::Serialize for ProtocolObject {\n");
    out.push_str("    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>\n");
    out.push_str("    where\n");
    out.push_str("        S: serde::Serializer,\n");
    out.push_str("    {\n");
    out.push_str("        match self {\n");
    for (_, _, variant_name, _) in &variants {
        out.push_str(&format!(
            "            Self::{variant_name}(value) => serde::Serialize::serialize(value, serializer),\n"
        ));
    }
    out.push_str(
        "            Self::Unknown(value) => serde::Serialize::serialize(value, serializer),\n",
    );
    out.push_str("        }\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");

    out.push_str("impl<'de> serde::Deserialize<'de> for ProtocolObject {\n");
    out.push_str("    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>\n");
    out.push_str("    where\n");
    out.push_str("        D: serde::Deserializer<'de>,\n");
    out.push_str("    {\n");
    out.push_str("        let value = <serde_json::Value as serde::Deserialize>::deserialize(deserializer)?;\n");
    out.push_str("        match value\n");
    out.push_str("            .get(\"id\")\n");
    out.push_str("            .and_then(serde_json::Value::as_str)\n");
    out.push_str("            .and_then(protocol_object_type_key)\n");
    out.push_str("        {\n");
    for (space, type_id, variant_name, _) in &variants {
        out.push_str(&format!(
            "            Some(({space}, {type_id})) => serde_json::from_value(value)\n"
        ));
        out.push_str(&format!("                .map(Self::{variant_name})\n"));
        out.push_str("                .map_err(serde::de::Error::custom),\n");
    }
    out.push_str("            _ => Ok(Self::Unknown(value)),\n");
    out.push_str("        }\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");

    out.push_str("fn protocol_object_type_key(id: &str) -> Option<(u32, u32)> {\n");
    out.push_str("    let mut parts = id.split('.');\n");
    out.push_str("    let space = parts.next()?.parse().ok()?;\n");
    out.push_str("    let object_type = parts.next()?.parse().ok()?;\n");
    out.push_str("    parts.next()?;\n");
    out.push_str("    if parts.next().is_some() {\n");
    out.push_str("        return None;\n");
    out.push_str("    }\n");
    out.push_str("    Some((space, object_type))\n");
    out.push_str("}\n\n");

    Ok(())
}

pub(crate) fn render_rpc_method(
    out: &mut String,
    protocol: &Protocol,
    api_name: &str,
    method: &RpcMethodDef,
    emitted: &mut BTreeSet<String>,
) -> Result<()> {
    let module_name = rust_field_name(&method.name);
    ensure_unique(emitted, &module_name, "rpc method module")?;

    let mut params = method.params.clone();
    params.sort_by_key(|param| param.index);
    // Only a trailing run of non-required parameters can be omitted positionally.
    let mut tail_start = params.len();
    while tail_start > 0 && !params[tail_start - 1].required {
        tail_start -= 1;
    }

    out.push_str(&format!(
        "    /// RPC `{api_name}.{}`.\n    pub mod {module_name} {{\n",
        method.name
    ));
    out.push_str(&format!(
        "        pub const API: &str = {};\n",
        rust_string_literal(api_name)
    ));
    out.push_str(&format!(
        "        pub const METHOD: &str = {};\n\n",
        rust_string_literal(&method.name)
    ));

    out.push_str(&format!(
        "        /// Positional parameters for `{api_name}.{}`.\n",
        method.name
    ));
    out.push_str("        #[derive(Debug, Clone, PartialEq)]\n");
    out.push_str("        pub struct Params {\n");
    let mut fields = Vec::new();
    let mut seen_fields = BTreeSet::new();
    for (index, param) in params.iter().enumerate() {
        let field_name = rust_field_name(&param.name);
        ensure_unique(&mut seen_fields, &field_name, "rpc parameter")?;
        let inner_ty = match &param.ty {
            TypeRef::Optional { inner } => inner.as_ref(),
            other => other,
        };
        let rendered = render_rpc_type_ref(protocol, inner_ty)?;
        let optional = index >= tail_start || matches!(param.ty, TypeRef::Optional { .. });
        if index >= tail_start {
            out.push_str(
                "            /// Omitted from the call when `None`; the node applies its default.\n",
            );
        }
        let is_copy = is_copy_primitive(&rendered);
        let field_ty = if optional {
            format!("Option<{rendered}>")
        } else {
            rendered
        };
        out.push_str(&format!("            pub {field_name}: {field_ty},\n"));
        fields.push((field_name, is_copy));
    }
    out.push_str("        }\n\n");

    out.push_str("        impl Params {\n");
    out.push_str("            /// The positional JSON parameter list for this call.\n");
    out.push_str(
        "            pub fn to_params_value(&self) -> Result<serde_json::Value, serde_json::Error> {\n",
    );
    if params.is_empty() {
        out.push_str("                Ok(serde_json::Value::Array(Vec::new()))\n");
    } else {
        let tail_len = params.len() - tail_start;
        if tail_start > 0 {
            let mutability = if tail_len > 0 { "mut " } else { "" };
            out.push_str(&format!("                let {mutability}params = vec![\n"));
            for (field_name, is_copy) in &fields[..tail_start] {
                // `Copy` values are passed by value; borrowing them would trip
                // clippy::needless_borrows_for_generic_args.
                let borrow = if *is_copy { "" } else { "&" };
                out.push_str(&format!(
                    "                    serde_json::to_value({borrow}self.{field_name})?,\n"
                ));
            }
            out.push_str("                ];\n");
        } else {
            out.push_str("                let mut params: Vec<serde_json::Value> = Vec::new();\n");
        }
        if tail_len > 0 {
            out.push_str(&format!(
                "                let tail: [Option<serde_json::Value>; {tail_len}] = [\n"
            ));
            for (field_name, _) in &fields[tail_start..] {
                out.push_str(&format!(
                    "                    match &self.{field_name} {{\n                        Some(value) => Some(serde_json::to_value(value)?),\n                        None => None,\n                    }},\n"
                ));
            }
            out.push_str("                ];\n");
            out.push_str(
                "                if let Some(last_provided) = tail.iter().rposition(|value| value.is_some()) {\n",
            );
            out.push_str(
                "                    for value in tail.into_iter().take(last_provided + 1) {\n",
            );
            out.push_str(
                "                        params.push(value.unwrap_or(serde_json::Value::Null));\n",
            );
            out.push_str("                    }\n");
            out.push_str("                }\n");
        }
        out.push_str("                Ok(serde_json::Value::Array(params))\n");
    }
    out.push_str("            }\n");
    out.push_str("        }\n\n");

    if api_name == "database" && method.name == "get_config" {
        render_config_rpc_types(out);
    }
    if api_name == "database" && method.name == "get_required_fees" {
        render_required_fee_rpc_types(out);
    }

    let returns = method.returns.clone().unwrap_or(TypeRef::Void);
    if matches!(returns, TypeRef::Void) {
        out.push_str("        /// This method returns no value.\n");
        out.push_str("        pub type Returns = ();\n\n");
        out.push_str(
            "        pub fn parse_returns(value: serde_json::Value) -> Result<Returns, serde_json::Error> {\n            let _ = value;\n            Ok(())\n        }\n",
        );
    } else {
        let rendered = render_rpc_type_ref(protocol, &returns)?;
        out.push_str(&format!("        pub type Returns = {rendered};\n\n"));
        out.push_str(
            "        pub fn parse_returns(value: serde_json::Value) -> Result<Returns, serde_json::Error> {\n            serde_json::from_value(value)\n        }\n",
        );
    }
    out.push_str("    }\n\n");
    Ok(())
}

/// Whether a rendered Rust type is a `Copy` primitive: those are passed to
/// `serde_json::to_value` by value, since borrowing them would trip
/// clippy::needless_borrows_for_generic_args in the generated code.
fn is_copy_primitive(rendered: &str) -> bool {
    matches!(
        rendered,
        "bool" | "u8" | "u16" | "u32" | "u64" | "i8" | "i16" | "i32" | "i64" | "f32" | "f64"
    )
}

/// Like [`render_type_ref`], but total: RPC signatures resolved from C++ may reference
/// types the spec does not model, so those degrade to `serde_json::Value` instead of
/// failing the whole generation, and byte types map to their hex JSON wire shape.
pub(crate) fn render_config_rpc_types(out: &mut String) {
    out.push_str("        /// Chain compile-time constants returned by `database.get_config`.\n");
    out.push_str("        ///\n");
    out.push_str("        /// The node returns an `fc::variant_object`, represented as a JSON object whose values\n");
    out.push_str("        /// remain dynamic because individual `GRAPHENE_*` constants mix strings and numbers.\n");
    out.push_str(
        "        pub type Config = std::collections::BTreeMap<String, serde_json::Value>;\n\n",
    );
}

pub(crate) fn render_required_fee_rpc_types(out: &mut String) {
    out.push_str("        /// Fee result returned by `database.get_required_fees`.\n");
    out.push_str("        ///\n");
    out.push_str("        /// Plain operations return a single [`Asset`]. `proposal_create_operation` returns\n");
    out.push_str(
        "        /// a pair of its own fee and the recursively priced proposed operations.\n",
    );
    out.push_str(
        "        #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]\n",
    );
    out.push_str("        #[serde(untagged)]\n");
    out.push_str("        pub enum RequiredFee {\n");
    out.push_str("            Asset(crate::generated::types::Asset),\n");
    out.push_str("            Proposal(ProposalRequiredFee),\n");
    out.push_str("        }\n\n");
    out.push_str("        /// Recursive fee shape for `proposal_create_operation`: `[proposal_fee, nested_fees]`.\n");
    out.push_str(
        "        #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]\n",
    );
    out.push_str("        pub struct ProposalRequiredFee(\n");
    out.push_str("            pub crate::generated::types::Asset,\n");
    out.push_str("            pub Vec<RequiredFee>,\n");
    out.push_str("        );\n\n");
}

pub(crate) fn render_rpc_type_ref(protocol: &Protocol, ty: &TypeRef) -> Result<String> {
    Ok(match ty {
        // On the JSON-RPC wire byte payloads are hex strings.
        TypeRef::Bytes | TypeRef::FixedBytes { .. } => "String".to_string(),
        TypeRef::Ref { name } if name == "config" => {
            "crate::generated::rpc::database::get_config::Config".to_string()
        }
        TypeRef::Ref { name } if name == "required_fee" => {
            "crate::generated::rpc::database::get_required_fees::RequiredFee".to_string()
        }
        TypeRef::ProtocolObjectUnion { .. } => "crate::generated::rpc::ProtocolObject".to_string(),
        TypeRef::AnyJson { .. } | TypeRef::Unsupported { .. } => "serde_json::Value".to_string(),
        TypeRef::Optional { inner } => {
            format!("Option<{}>", render_rpc_type_ref(protocol, inner)?)
        }
        TypeRef::Vector { inner } | TypeRef::Set { inner, .. } => {
            format!("Vec<{}>", render_rpc_type_ref(protocol, inner)?)
        }
        TypeRef::Map { key, value, .. } | TypeRef::FlatMap { key, value, .. } => format!(
            "Vec<({}, {})>",
            render_rpc_type_ref(protocol, key)?,
            render_rpc_type_ref(protocol, value)?
        ),
        TypeRef::Pair { first, second } => format!(
            "({}, {})",
            render_rpc_type_ref(protocol, first)?,
            render_rpc_type_ref(protocol, second)?
        ),
        TypeRef::Ref { name } if !rpc_ref_target_exists(protocol, name) => {
            "serde_json::Value".to_string()
        }
        TypeRef::StaticVariantRef { name }
            if !protocol
                .static_variants
                .iter()
                .any(|variant| &variant.name == name) =>
        {
            "serde_json::Value".to_string()
        }
        other => render_type_ref(protocol, other)?,
    })
}

pub(crate) fn rpc_ref_target_exists(protocol: &Protocol, name: &str) -> bool {
    protocol
        .structs
        .iter()
        .any(|struct_def| struct_def.name == name)
        || protocol
            .operations
            .iter()
            .any(|operation| operation.name == name)
        || protocol.enums.iter().any(|enum_def| enum_def.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::test_support::minimal_protocol;
    use open_graphene_json_schema::RpcParamDef;

    #[test]
    fn renders_typed_rpc_params_and_return_parser() {
        let mut protocol = minimal_protocol();
        protocol.rpc_methods.push(RpcMethodDef {
            name: "get_sample_objects".to_string(),
            api_class: "database_api".to_string(),
            api_name: Some("database".to_string()),
            params: vec![
                RpcParamDef {
                    index: 0,
                    name: "object_ids".to_string(),
                    ty: TypeRef::Vector {
                        inner: Box::new(TypeRef::ObjectId),
                    },
                    required: true,
                    default_value: None,
                    nullable: false,
                    source: None,
                    support: None,
                },
                RpcParamDef {
                    index: 1,
                    name: "maybe_subscribe".to_string(),
                    ty: TypeRef::Optional {
                        inner: Box::new(TypeRef::Bool),
                    },
                    required: true,
                    default_value: None,
                    nullable: true,
                    source: None,
                    support: None,
                },
                RpcParamDef {
                    index: 2,
                    name: "with_details".to_string(),
                    ty: TypeRef::Bool,
                    required: false,
                    default_value: Some("false".to_string()),
                    nullable: false,
                    source: None,
                    support: None,
                },
            ],
            returns: Some(TypeRef::Vector {
                inner: Box::new(TypeRef::AnyJson {
                    reason: None,
                    source: None,
                }),
            }),
            is_subscription: false,
            notices: vec![],
            binding_hints: None,
            source: None,
            support: None,
        });

        let output = render_rpc(&protocol).expect("render rpc");

        assert!(output.contains("pub mod database"));
        assert!(output.contains("pub mod get_sample_objects"));
        assert!(output.contains("pub const API: &str = \"database\";"));
        assert!(output.contains("pub const METHOD: &str = \"get_sample_objects\";"));
        assert!(output.contains("pub object_ids: Vec<crate::generated::ids::ObjectId>"));
        assert!(output.contains("pub maybe_subscribe: Option<bool>"));
        assert!(output.contains("pub with_details: Option<bool>"));
        assert!(output.contains("let mut params = vec![\n"));
        assert!(output.contains("serde_json::to_value(&self.object_ids)?,"));
        // Copy-typed values are passed by value, not borrowed.
        assert!(output.contains("serde_json::to_value(self.maybe_subscribe)?,"));
        assert!(output.contains("if let Some(last_provided) = tail.iter().rposition"));
        assert!(output.contains("params.push(value.unwrap_or(serde_json::Value::Null));"));
        assert!(output.contains("pub type Returns = Vec<serde_json::Value>;"));
        assert!(output.contains("pub fn parse_returns(value: serde_json::Value)"));
    }

    #[test]
    fn renders_protocol_object_union_rpc_shape() {
        let mut protocol = minimal_protocol();
        protocol
            .object_types
            .push(open_graphene_json_schema::ObjectTypeDef {
                object_type: "account".to_string(),
                cpp_alias: "account_object_type".to_string(),
                object_space: Some(1),
                type_id: Some(2),
                struct_ref: Some("account_object".to_string()),
                source: None,
                support: None,
            });
        protocol.rpc_methods.push(RpcMethodDef {
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
        });

        let output = render_rpc(&protocol).expect("render rpc");

        assert!(output.contains("pub enum ProtocolObject"));
        assert!(output.contains("Account(Box<crate::generated::types::AccountObject>)"));
        assert!(output.contains("Some((1, 2)) => serde_json::from_value(value)"));
        assert!(output.contains(".map(Self::Account)"));
        assert!(output.contains("Unknown(serde_json::Value)"));
        assert!(
            output
                .contains("pub type Returns = Vec<Option<crate::generated::rpc::ProtocolObject>>;")
        );
    }

    #[test]
    fn renders_config_rpc_shape() {
        let mut protocol = minimal_protocol();
        protocol.rpc_methods.push(RpcMethodDef {
            name: "get_config".to_string(),
            api_class: "database_api".to_string(),
            api_name: Some("database".to_string()),
            params: vec![],
            returns: Some(TypeRef::Ref {
                name: "config".to_string(),
            }),
            is_subscription: false,
            notices: vec![],
            binding_hints: None,
            source: None,
            support: None,
        });

        let output = render_rpc(&protocol).expect("render rpc");

        assert!(
            output.contains(
                "pub type Config = std::collections::BTreeMap<String, serde_json::Value>;"
            )
        );
        assert!(
            output.contains(
                "pub type Returns = crate::generated::rpc::database::get_config::Config;"
            )
        );
    }

    #[test]
    fn renders_required_fee_rpc_shape() {
        let mut protocol = minimal_protocol();
        protocol.rpc_methods.push(RpcMethodDef {
            name: "get_required_fees".to_string(),
            api_class: "database_api".to_string(),
            api_name: Some("database".to_string()),
            params: vec![],
            returns: Some(TypeRef::Vector {
                inner: Box::new(TypeRef::Ref {
                    name: "required_fee".to_string(),
                }),
            }),
            is_subscription: false,
            notices: vec![],
            binding_hints: None,
            source: None,
            support: None,
        });

        let output = render_rpc(&protocol).expect("render rpc");

        assert!(output.contains("pub enum RequiredFee"));
        assert!(output.contains("Asset(crate::generated::types::Asset)"));
        assert!(output.contains("Proposal(ProposalRequiredFee)"));
        assert!(output.contains("pub struct ProposalRequiredFee("));
        assert!(output.contains("pub Vec<RequiredFee>"));
        assert!(output.contains(
            "pub type Returns = Vec<crate::generated::rpc::database::get_required_fees::RequiredFee>;"
        ));
    }

    #[test]
    fn rpc_renderer_emits_typed_params_and_returns() {
        let mut protocol = minimal_protocol();
        protocol.structs.push(open_graphene_json_schema::StructDef {
            name: "account_object".to_string(),
            source_name: None,
            kind: StructKind::Struct,
            wire_tag: None,
            fields: vec![],
            support: None,
        });
        protocol.rpc_methods.push(rpc_method(
            "get_accounts",
            vec![
                rpc_param(
                    0,
                    "account_names_or_ids",
                    TypeRef::Vector {
                        inner: Box::new(TypeRef::String),
                    },
                    true,
                ),
                rpc_param(
                    1,
                    "subscribe",
                    TypeRef::Optional {
                        inner: Box::new(TypeRef::Bool),
                    },
                    false,
                ),
            ],
            Some(TypeRef::Vector {
                inner: Box::new(TypeRef::Optional {
                    inner: Box::new(TypeRef::Ref {
                        name: "account_object".to_string(),
                    }),
                }),
            }),
        ));

        let rendered = render_rpc(&protocol).expect("render rpc");

        assert!(rendered.contains("pub mod database {"));
        assert!(rendered.contains("pub mod get_accounts {"));
        assert!(rendered.contains("pub const API: &str = \"database\";"));
        assert!(rendered.contains("pub const METHOD: &str = \"get_accounts\";"));
        assert!(rendered.contains("pub account_names_or_ids: Vec<String>,"));
        assert!(rendered.contains("pub subscribe: Option<bool>,"));
        // The optional tail is dropped positionally when not provided.
        assert!(rendered.contains("tail.iter().rposition(|value| value.is_some())"));
        assert!(
            rendered.contains(
                "pub type Returns = Vec<Option<crate::generated::types::AccountObject>>;"
            )
        );
    }

    #[test]
    fn rpc_renderer_degrades_unmodeled_types_and_maps_bytes_to_hex_strings() {
        let mut protocol = minimal_protocol();
        protocol.rpc_methods.push(rpc_method(
            "range_get_info",
            vec![rpc_param(0, "proof", TypeRef::Bytes, true)],
            Some(TypeRef::Ref {
                name: "not_in_spec".to_string(),
            }),
        ));

        let rendered = render_rpc(&protocol).expect("render rpc");

        assert!(rendered.contains("pub proof: String,"));
        assert!(rendered.contains("pub type Returns = serde_json::Value;"));
    }

    #[test]
    fn rpc_renderer_emits_unit_returns_for_void_methods() {
        let mut protocol = minimal_protocol();
        protocol.rpc_methods.push(rpc_method(
            "broadcast_transaction",
            vec![],
            Some(TypeRef::Void),
        ));

        let rendered = render_rpc(&protocol).expect("render rpc");

        assert!(rendered.contains("pub type Returns = ();"));
        assert!(rendered.contains("Ok(serde_json::Value::Array(Vec::new()))"));
    }

    fn rpc_method(
        name: &str,
        params: Vec<open_graphene_json_schema::RpcParamDef>,
        returns: Option<TypeRef>,
    ) -> RpcMethodDef {
        RpcMethodDef {
            name: name.to_string(),
            api_class: "database_api".to_string(),
            api_name: Some("database".to_string()),
            params,
            returns,
            is_subscription: false,
            notices: vec![],
            binding_hints: None,
            source: None,
            support: None,
        }
    }

    fn rpc_param(
        index: u32,
        name: &str,
        ty: TypeRef,
        required: bool,
    ) -> open_graphene_json_schema::RpcParamDef {
        open_graphene_json_schema::RpcParamDef {
            index,
            name: name.to_string(),
            ty,
            required,
            default_value: None,
            nullable: !required,
            source: None,
            support: None,
        }
    }
}
