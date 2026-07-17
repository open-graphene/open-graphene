use super::*;

pub(crate) fn render_static_variants(
    protocol: &Protocol,
    no_recursion_cuts: &SchemaNoRecursionCuts,
) -> Result<String> {
    let mut out = generated_header(protocol, "static variant enums");
    out.push_str("// Static variants use Graphene JSON wire format: [tag, value].\n\n");
    render_static_variant_schema_helpers(&mut out);

    let mut emitted = BTreeSet::new();
    for variant in sorted_static_variants(&protocol.static_variants) {
        let name = rust_type_name(&variant.name);
        ensure_unique(&mut emitted, &name, "static variant")?;
        render_static_variant(&mut out, protocol, &variant, no_recursion_cuts)?;
    }

    Ok(out)
}

pub(crate) fn render_static_variant(
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
    // ToSchema ręcznie (niżej), nie z derive: wire format to krotka
    // `[tag, value]`, a derive na enumie opisałby unię obiektów — klienci
    // walidujący odpowiedzi (zod) odrzucaliby poprawne payloady.
    out.push_str("#[derive(Debug, Clone, PartialEq)]\n");
    out.push_str(&format!("pub enum {enum_name} {{\n"));

    let mut variants = variant.variants.clone();
    variants.sort_by_key(|arm| arm.tag);
    let mut emitted = BTreeSet::new();
    let mut method_names = BTreeSet::new();
    let mut rendered_arms = Vec::new();
    let mut spec_arm_names = Vec::new();
    let mut schema_arms = Vec::new();
    for arm in variants {
        let variant_name = rust_variant_name(&arm.name);
        ensure_unique(&mut emitted, &variant_name, "static variant arm")?;
        let method_name =
            static_variant_constructor_name(&enum_name, &arm.name, &mut method_names)?;
        let ty = render_type_ref(protocol, &arm.ty)?;
        let _ = no_recursion_cuts;
        out.push_str(&format!("    {variant_name}(Box<{ty}>),\n"));
        spec_arm_names.push((variant_name.clone(), arm.name.clone()));
        schema_arms.push((arm.tag, arm.name, ty.clone()));
        rendered_arms.push((arm.tag, variant_name, ty, method_name));
    }

    out.push_str("}\n\n");
    render_static_variant_schema_impl(out, protocol, &enum_name, &schema_arms);
    if enum_name == "Operation" {
        render_static_variant_constructor_impl(out, &enum_name, &rendered_arms);
        render_static_variant_accessor_impl(out, &enum_name, &rendered_arms);
        render_operation_metadata_impl(out, protocol, &spec_arm_names);
    }
    if enum_name == "FutureExtensions" {
        render_future_extensions_empty_impl(out, &rendered_arms);
    }
    render_static_variant_serialize_impl(out, &enum_name, &rendered_arms);
    render_static_variant_deserialize_impl(out, &enum_name, &rendered_arms);
    Ok(())
}

/// Shared runtime scaffolding for the manual static-variant schemas: one
/// `[tag, value]` tuple schema per arm, with payloads either `$ref`ed (named
/// protocol types, wrapped in `allOf` because `prefixItems` holds plain
/// schemas) or inlined (primitives).
pub(crate) fn render_static_variant_schema_helpers(out: &mut String) {
    out.push_str(
        "#[allow(dead_code)]\nfn static_variant_arm_schema(\n    tag: u32,\n    arm_name: &str,\n    payload: utoipa::openapi::schema::Schema,\n) -> utoipa::openapi::schema::Schema {\n    utoipa::openapi::schema::Schema::Array(\n        utoipa::openapi::schema::ArrayBuilder::new()\n            .items(utoipa::openapi::schema::ArrayItems::False)\n            .prefix_items([\n                utoipa::openapi::schema::Schema::Object(\n                    utoipa::openapi::schema::ObjectBuilder::new()\n                        .schema_type(utoipa::openapi::schema::SchemaType::Type(\n                            utoipa::openapi::schema::Type::Integer,\n                        ))\n                        .enum_values(Some([tag]))\n                        .description(Some(format!(\"`{arm_name}` tag\")))\n                        .build(),\n                ),\n                payload,\n            ])\n            .min_items(Some(2))\n            .max_items(Some(2))\n            .description(Some(format!(\"`{arm_name}` as `[{tag}, value]`\")))\n            .build(),\n    )\n}\n\n#[allow(dead_code)]\nfn static_variant_ref_payload<T: utoipa::ToSchema>() -> utoipa::openapi::schema::Schema {\n    utoipa::openapi::schema::Schema::AllOf(\n        utoipa::openapi::schema::AllOfBuilder::new()\n            .item(utoipa::openapi::Ref::from_schema_name(T::name()))\n            .build(),\n    )\n}\n\n#[allow(dead_code)]\nfn static_variant_inline_payload<T: utoipa::PartialSchema>() -> utoipa::openapi::schema::Schema {\n    match T::schema() {\n        utoipa::openapi::RefOr::T(schema) => schema,\n        utoipa::openapi::RefOr::Ref(reference) => utoipa::openapi::schema::Schema::AllOf(\n            utoipa::openapi::schema::AllOfBuilder::new()\n                .item(reference)\n                .build(),\n        ),\n    }\n}\n\n#[allow(dead_code)]\nfn static_variant_any_payload() -> utoipa::openapi::schema::Schema {\n    utoipa::openapi::schema::Schema::Object(\n        utoipa::openapi::schema::ObjectBuilder::new()\n            .schema_type(utoipa::openapi::schema::SchemaType::AnyValue)\n            .build(),\n    )\n}\n\n",
    );
}

/// The Rust expression producing the OpenAPI schema for one arm payload.
fn static_variant_payload_expr(ty: &str) -> String {
    if ty == "()" || ty == "serde_json::Value" || ty.starts_with('(') {
        // Void, free-form JSON and pair payloads have no precise schema.
        "static_variant_any_payload()".to_string()
    } else if ty == "Vec<u8>" {
        // Byte payloads are hex strings on the wire (see the Serialize impl).
        "static_variant_inline_payload::<String>()".to_string()
    } else if ty.starts_with("crate::generated::") {
        format!("static_variant_ref_payload::<{ty}>()")
    } else {
        format!("static_variant_inline_payload::<{ty}>()")
    }
}

/// Named component types mentioned anywhere in a rendered arm type (also
/// inside `Vec<...>` / pairs); these must be registered as schema
/// dependencies because the tuple schema only references them.
fn named_schema_dependencies(ty: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut rest = ty;
    while let Some(position) = rest.find("crate::generated::") {
        let tail = &rest[position..];
        let end = tail
            .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == ':'))
            .unwrap_or(tail.len());
        found.push(tail[..end].to_string());
        rest = &tail[end..];
    }
    found
}

pub(crate) fn render_static_variant_schema_impl(
    out: &mut String,
    protocol: &Protocol,
    enum_name: &str,
    schema_arms: &[(u32, String, String)],
) {
    let schema_name = openapi_schema_name(protocol, enum_name);

    out.push_str(&format!(
        "impl utoipa::PartialSchema for {enum_name} {{\n    fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {{\n"
    ));
    if schema_arms.is_empty() {
        out.push_str("        utoipa::openapi::schema::Schema::Array(\n            utoipa::openapi::schema::ArrayBuilder::new()\n                .items(\n                    utoipa::openapi::schema::ObjectBuilder::new()\n                        .schema_type(utoipa::openapi::schema::SchemaType::AnyValue)\n                        .build(),\n                )\n                .description(Some(\"Graphene static variant wire tuple: [tag, value].\"))\n                .build(),\n        )\n        .into()\n    }\n}\n\n");
    } else {
        out.push_str("        utoipa::openapi::schema::Schema::OneOf(\n            utoipa::openapi::schema::OneOfBuilder::new()\n                .description(Some(\n                    \"Graphene static variant wire tuple: [tag, value].\",\n                ))\n");
        for (tag, spec_name, ty) in schema_arms {
            let payload = static_variant_payload_expr(ty);
            out.push_str(&format!(
                "                .item(static_variant_arm_schema(\n                    {tag},\n                    {},\n                    {payload},\n                ))\n",
                rust_string_literal(spec_name)
            ));
        }
        out.push_str("                .build(),\n        )\n        .into()\n    }\n}\n\n");
    }

    out.push_str(&format!(
        "impl utoipa::ToSchema for {enum_name} {{\n    fn name() -> std::borrow::Cow<'static, str> {{\n        std::borrow::Cow::Borrowed(\"{schema_name}\")\n    }}\n"
    ));
    let mut dependencies = BTreeSet::new();
    for (_, _, ty) in schema_arms {
        dependencies.extend(named_schema_dependencies(ty));
    }
    if !dependencies.is_empty() {
        out.push_str("\n    fn schemas(\n        schemas: &mut Vec<(\n            String,\n            utoipa::openapi::RefOr<utoipa::openapi::schema::Schema>,\n        )>,\n    ) {\n        // The tuple schema only references payload components; register them\n        // here. The guard cuts self-referential dependency walks (proposal\n        // operations embed operations again).\n        thread_local! {\n            static WALKING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };\n        }\n        if WALKING.replace(true) {\n            return;\n        }\n");
        for dependency in &dependencies {
            out.push_str(&format!(
                "        schemas.push((\n            <{dependency} as utoipa::ToSchema>::name().to_string(),\n            <{dependency} as utoipa::PartialSchema>::schema(),\n        ));\n        <{dependency} as utoipa::ToSchema>::schemas(schemas);\n"
            ));
        }
        out.push_str("        WALKING.set(false);\n    }\n");
    }
    out.push_str("}\n\n");
}

pub(crate) fn render_static_variant_constructor_impl(
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

pub(crate) fn render_static_variant_accessor_impl(
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

/// Uniform metadata on the `Operation` static variant: the protocol operation
/// name, the chain-emitted (virtual) flag, and `fee` access. `fee`/`set_fee`
/// are only emitted when every operation in the spec carries a `fee: asset`
/// field, so the accessors can stay total.
pub(crate) fn render_operation_metadata_impl(
    out: &mut String,
    protocol: &Protocol,
    arms: &[(String, String)],
) {
    let operation_def = |spec_name: &str| {
        protocol
            .operations
            .iter()
            .find(|operation| operation.name == spec_name)
    };

    out.push_str("impl Operation {\n");

    out.push_str("    /// The protocol operation name (e.g. `transfer`).\n");
    out.push_str("    pub fn name(&self) -> &'static str {\n");
    out.push_str("        match self {\n");
    for (variant_name, spec_name) in arms {
        let operation_name = spec_name.strip_suffix("_operation").unwrap_or(spec_name);
        out.push_str(&format!(
            "            Self::{variant_name}(_) => {},\n",
            rust_string_literal(operation_name)
        ));
    }
    out.push_str("        }\n    }\n\n");

    out.push_str(
        "    /// Whether the chain emits this operation itself; virtual operations can never be broadcast.\n",
    );
    out.push_str("    pub fn is_virtual(&self) -> bool {\n");
    out.push_str("        match self {\n");
    for (variant_name, spec_name) in arms {
        let is_virtual = operation_def(spec_name).is_some_and(|operation| operation.is_virtual);
        out.push_str(&format!(
            "            Self::{variant_name}(_) => {is_virtual},\n"
        ));
    }
    out.push_str("        }\n    }\n");

    let every_operation_has_asset_fee = !arms.is_empty()
        && arms.iter().all(|(_, spec_name)| {
            operation_def(spec_name).is_some_and(|operation| {
                operation.fields.iter().any(|field| {
                    field.name == "fee"
                        && matches!(&field.ty, TypeRef::Ref { name } if name == "asset")
                })
            })
        });
    if every_operation_has_asset_fee {
        out.push_str(
            "\n    /// The operation fee (every operation in this protocol carries one).\n",
        );
        out.push_str("    pub fn fee(&self) -> &crate::generated::types::Asset {\n");
        out.push_str("        match self {\n");
        for (variant_name, _) in arms {
            out.push_str(&format!(
                "            Self::{variant_name}(value) => &value.fee,\n"
            ));
        }
        out.push_str("        }\n    }\n\n");

        out.push_str("    /// Write the node-priced fee into the operation.\n");
        out.push_str("    pub fn set_fee(&mut self, fee: crate::generated::types::Asset) {\n");
        out.push_str("        match self {\n");
        for (variant_name, _) in arms {
            out.push_str(&format!(
                "            Self::{variant_name}(value) => value.fee = fee,\n"
            ));
        }
        out.push_str("        }\n    }\n");
    }

    out.push_str("}\n\n");
}

pub(crate) fn render_future_extensions_empty_impl(
    out: &mut String,
    arms: &[(u32, String, String, String)],
) {
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

pub(crate) fn render_static_variant_serialize_impl(
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

pub(crate) fn render_static_variant_deserialize_impl(
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

pub(crate) fn static_variant_constructor_name(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::test_support::minimal_protocol;

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
    fn operation_static_variant_emits_name_virtual_and_fee_accessors() {
        let mut protocol = minimal_protocol();
        let asset_fee_field = FieldDef {
            index: 0,
            name: "fee".to_string(),
            ty: TypeRef::Ref {
                name: "asset".to_string(),
            },
            source: None,
            support: None,
        };
        protocol.operations.push(OperationDef {
            name: "transfer_operation".to_string(),
            wire_tag: 0,
            fields: vec![asset_fee_field.clone()],
            is_virtual: false,
            source: None,
            support: None,
        });
        protocol.operations.push(OperationDef {
            name: "fill_order_operation".to_string(),
            wire_tag: 4,
            fields: vec![asset_fee_field],
            is_virtual: true,
            source: None,
            support: None,
        });
        let variant = StaticVariantDef {
            name: "operation".to_string(),
            kind: "static_variant".to_string(),
            json: "tagged_tuple".to_string(),
            fc: "static_variant".to_string(),
            variants: vec![
                open_graphene_json_schema::StaticVariantArmDef {
                    tag: 0,
                    name: "transfer_operation".to_string(),
                    ty: TypeRef::String,
                    support: None,
                },
                open_graphene_json_schema::StaticVariantArmDef {
                    tag: 4,
                    name: "fill_order_operation".to_string(),
                    ty: TypeRef::String,
                    support: None,
                },
            ],
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

        assert!(out.contains("pub fn name(&self) -> &'static str"));
        assert!(out.contains("Self::TransferOperation(_) => \"transfer\","));
        assert!(out.contains("Self::FillOrderOperation(_) => \"fill_order\","));
        assert!(out.contains("pub fn is_virtual(&self) -> bool"));
        assert!(out.contains("Self::TransferOperation(_) => false,"));
        assert!(out.contains("Self::FillOrderOperation(_) => true,"));
        assert!(out.contains("pub fn fee(&self) -> &crate::generated::types::Asset"));
        assert!(out.contains("pub fn set_fee(&mut self, fee: crate::generated::types::Asset)"));
        assert!(out.contains("Self::TransferOperation(value) => value.fee = fee,"));
    }

    #[test]
    fn operation_fee_accessors_are_skipped_when_an_operation_lacks_an_asset_fee() {
        let mut protocol = minimal_protocol();
        protocol.operations.push(OperationDef {
            name: "transfer_operation".to_string(),
            wire_tag: 0,
            fields: vec![],
            is_virtual: false,
            source: None,
            support: None,
        });
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

        assert!(out.contains("pub fn name(&self) -> &'static str"));
        assert!(out.contains("pub fn is_virtual(&self) -> bool"));
        assert!(!out.contains("pub fn fee("));
        assert!(!out.contains("pub fn set_fee("));
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
}
