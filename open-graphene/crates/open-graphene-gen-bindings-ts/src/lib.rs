//! Deterministic TypeScript emission. Protocol definitions come exclusively from
//! the shared IR; the hand-written runtime supplies checked JSON primitives.
use open_graphene_codegen_common::{
    DEFAULT_FIELDS, LEGACY_ACCESS_OPERATIONS, PROFILE_VERSION, RPC_SYNTHETIC_REFS,
};
use open_graphene_json_schema::{FieldDef, JsonShape, Protocol, TypeRef};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
    fs,
    path::Path,
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
mod fc;
fn q(value: &str) -> String {
    serde_json::to_string(value).expect("string")
}
fn name(value: &str) -> String {
    let mut result = value
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(|s| format!("{}{}", s[..1].to_ascii_uppercase(), &s[1..]))
        .collect::<String>();
    if result.starts_with(|c: char| c.is_ascii_digit()) {
        result.insert(0, 'N');
    }
    result
}
fn unique(set: &mut BTreeSet<String>, value: String, scope: &str) -> Result<()> {
    if value.is_empty() || !set.insert(value.clone()) {
        return Err(format!("duplicate/empty {scope}: {value}").into());
    }
    Ok(())
}

pub fn validate(protocol: &Protocol) -> Result<()> {
    if protocol.schema_version != 1 {
        return Err(format!("unsupported schemaVersion {}", protocol.schema_version).into());
    }
    let mut names = BTreeSet::from([
        "ObjectId".into(),
        "PublicKey".into(),
        "TimePointSec".into(),
        "VoteId".into(),
        "Signature".into(),
        "ProtocolObject".into(),
        "Config".into(),
        "RequiredFee".into(),
    ]);
    let mut known = BTreeSet::new();
    let mut operation_names = BTreeSet::new();
    let mut tags = BTreeSet::new();
    for operation in &protocol.operations {
        unique(&mut operation_names, operation.name.clone(), "operation")?;
        if !tags.insert(operation.wire_tag) {
            return Err("duplicate operation tag".into());
        }
        validate_fields(&operation.fields)?;
    }
    for def in &protocol.structs {
        unique(&mut known, def.name.clone(), "type")?;
        unique(&mut names, name(&def.name), "TypeScript symbol")?;
        validate_fields(&def.fields)?;
        if let Some(op) = protocol.operations.iter().find(|op| op.name == def.name)
            && op.fields != def.fields
        {
            return Err(format!("conflicting operation/struct {}", op.name).into());
        }
    }
    for op in &protocol.operations {
        if !known.contains(&op.name) {
            unique(&mut known, op.name.clone(), "type")?;
            unique(&mut names, name(&op.name), "TypeScript symbol")?;
        }
    }
    for def in &protocol.enums {
        unique(&mut known, def.name.clone(), "type")?;
        unique(&mut names, name(&def.name), "TypeScript symbol")?;
        if def
            .support
            .as_ref()
            .is_some_and(|s| s.status == open_graphene_json_schema::SupportStatus::Unsupported)
        {
            return Err(format!("enum {} has unresolved source values", def.name).into());
        }
    }
    for def in &protocol.static_variants {
        unique(&mut known, def.name.clone(), "type")?;
        unique(&mut names, name(&def.name), "TypeScript symbol")?;
        let mut tags = BTreeSet::new();
        for arm in &def.variants {
            if !tags.insert(arm.tag) {
                return Err(format!("duplicate variant tag in {}", def.name).into());
            }
        }
    }
    if let Some(operation) = protocol
        .static_variants
        .iter()
        .find(|v| v.name == "operation")
    {
        for op in &protocol.operations {
            if !operation
                .variants
                .iter()
                .any(|a| a.tag == op.wire_tag && a.name == op.name)
            {
                return Err(format!("operation tag mismatch: {}", op.name).into());
            }
        }
    }
    let mut objects = BTreeSet::new();
    let mut object_tags = BTreeSet::new();
    for def in &protocol.object_types {
        unique(&mut objects, def.object_type.clone(), "object type")?;
        if def.object_type != "vote" {
            unique(
                &mut names,
                format!("{}Id", name(&def.object_type)),
                "TypeScript symbol",
            )?;
        }
        if let (Some(space), Some(ty)) = (def.object_space, def.type_id) {
            if space > 255 || ty > 255 || !object_tags.insert((space, ty)) {
                return Err("invalid/duplicate object type ID".into());
            }
        } else if def.struct_ref.is_some() {
            return Err("object with structRef needs space/type".into());
        }
        if let Some(reference) = &def.struct_ref
            && !known.contains(reference)
        {
            return Err(format!("missing object struct {reference}").into());
        }
    }
    let mut methods = BTreeSet::new();
    for method in &protocol.rpc_methods {
        unique(
            &mut methods,
            format!(
                "{}.{}",
                method.api_name.as_deref().unwrap_or(&method.api_class),
                method.name
            ),
            "RPC method",
        )?;
        let mut indexes = BTreeSet::new();
        let mut params = BTreeSet::new();
        for p in &method.params {
            if !indexes.insert(p.index) {
                return Err("duplicate RPC parameter index".into());
            }
            unique(&mut params, p.name.clone(), "RPC parameter")?;
        }
        if indexes.iter().copied().ne(0..method.params.len() as u32) {
            return Err("non-contiguous RPC parameter indexes".into());
        }
    }
    fn visit(value: &Value, known: &BTreeSet<String>, objects: &BTreeSet<String>) -> Result<()> {
        match value {
            Value::Object(map) => {
                match map.get("kind").and_then(Value::as_str) {
                    Some("ref" | "static_variant_ref") => {
                        let reference = map["name"].as_str().ok_or("invalid reference")?;
                        if !known.contains(reference) && !RPC_SYNTHETIC_REFS.contains(&reference) {
                            return Err(format!("unresolved reference: {reference}").into());
                        }
                    }
                    Some("protocol_object_id") => {
                        let object = map["objectType"]
                            .as_str()
                            .ok_or("invalid object reference")?;
                        if !objects.contains(object) && object != "vote" {
                            return Err(format!("unresolved object type: {object}").into());
                        }
                    }
                    _ => {}
                }
                for v in map.values() {
                    visit(v, known, objects)?;
                }
            }
            Value::Array(items) => {
                for item in items {
                    visit(item, known, objects)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    visit(&serde_json::to_value(protocol)?, &known, &objects)
}
fn validate_fields(fields: &[FieldDef]) -> Result<()> {
    let mut names = BTreeSet::new();
    let mut indexes = BTreeSet::new();
    for field in fields {
        unique(&mut names, field.name.clone(), "field")?;
        if !indexes.insert(field.index) {
            return Err("duplicate field index".into());
        }
    }
    if indexes.iter().copied().ne(0..fields.len() as u32) {
        return Err("non-contiguous field indexes".into());
    }
    Ok(())
}

fn ts(ty: &TypeRef) -> String {
    match ty {
        TypeRef::Void => "Readonly<Record<string, never>>".into(),
        TypeRef::Bool => "boolean".into(),
        TypeRef::Uint8 | TypeRef::Uint16 | TypeRef::Uint32 | TypeRef::Int32 { .. } => {
            "number".into()
        }
        TypeRef::Int64 { .. }
        | TypeRef::Uint64 { .. }
        | TypeRef::Uint128 { .. }
        | TypeRef::UnsignedVarint
        | TypeRef::CallbackHandle => "bigint".into(),
        TypeRef::String | TypeRef::Address | TypeRef::TimePoint | TypeRef::FixedHex { .. } => {
            "string".into()
        }
        TypeRef::Bytes | TypeRef::FixedBytes { .. } | TypeRef::Signature => "Uint8Array".into(),
        TypeRef::PublicKey { .. } => "PublicKey".into(),
        TypeRef::TimePointSec => "TimePointSec".into(),
        TypeRef::VoteId => "VoteId".into(),
        TypeRef::ObjectId => "ObjectId".into(),
        TypeRef::ProtocolObjectId { object_type } => format!("{}Id", name(object_type)),
        TypeRef::ProtocolObjectUnion { .. } => "ProtocolObject".into(),
        TypeRef::Optional { inner } => format!("({} | null)", ts(inner)),
        TypeRef::Vector { inner } | TypeRef::Set { inner, .. } => {
            format!("ReadonlyArray<{}>", ts(inner))
        }
        TypeRef::Map { key, value, .. } | TypeRef::FlatMap { key, value, .. } => {
            format!("ReadonlyArray<readonly [{}, {}]>", ts(key), ts(value))
        }
        TypeRef::Pair { first, second } => format!("readonly [{}, {}]", ts(first), ts(second)),
        TypeRef::Ref { name: n } | TypeRef::StaticVariantRef { name: n } => name(n),
        TypeRef::AnyJson { .. } => "unknown".into(),
        TypeRef::Unsupported { .. } => "never".into(),
    }
}
fn codec_expr(ty: &TypeRef, protocol: &Protocol) -> String {
    match ty {
        TypeRef::Void => "c.voidValue".into(),
        TypeRef::Bool => "c.bool".into(),
        TypeRef::Uint8 => "c.smallInteger(0, 255)".into(),
        TypeRef::Uint16 => "c.smallInteger(0, 65535)".into(),
        TypeRef::Uint32 => "c.smallInteger(0, 4294967295)".into(),
        TypeRef::Int32 { .. } => "c.smallInteger(-2147483648, 2147483647)".into(),
        TypeRef::Int64 { json, .. } | TypeRef::Uint64 { json, .. } => format!(
            "c.wideInteger(64, {}, {})",
            matches!(ty, TypeRef::Int64 { .. }),
            matches!(json, Some(JsonShape::DecimalString | JsonShape::String))
        ),
        TypeRef::Uint128 { .. } => "c.wideInteger(128, false, true)".into(),
        TypeRef::UnsignedVarint | TypeRef::CallbackHandle => "c.wideInteger(64, false)".into(),
        TypeRef::String | TypeRef::Address | TypeRef::TimePoint => "c.text".into(),
        TypeRef::Bytes => "c.bytes()".into(),
        TypeRef::FixedBytes { bytes } => format!("c.bytes({bytes})"),
        TypeRef::FixedHex { bytes } => format!("c.fixedHex({bytes})"),
        TypeRef::Signature => "c.bytes(65)".into(),
        TypeRef::PublicKey { chain_prefix, .. } => format!(
            "c.publicKey({})",
            q(chain_prefix
                .as_deref()
                .unwrap_or(&protocol.chain.public_key_prefix))
        ),
        TypeRef::TimePointSec => "c.timePointSec".into(),
        TypeRef::VoteId => "c.voteId".into(),
        TypeRef::ObjectId => "c.objectId()".into(),
        TypeRef::ProtocolObjectId { object_type } if object_type == "vote" => "c.voteId".into(),
        TypeRef::ProtocolObjectId { object_type } => {
            let object = protocol
                .object_types
                .iter()
                .find(|o| o.object_type == *object_type)
                .expect("validated");
            format!(
                "c.objectId({}, {})",
                object
                    .object_space
                    .map_or("undefined".into(), |n| n.to_string()),
                object.type_id.map_or("undefined".into(), |n| n.to_string())
            )
        }
        TypeRef::ProtocolObjectUnion { .. } => "ProtocolObjectCodec".into(),
        TypeRef::Optional { inner } => format!("c.optional({})", codec_expr(inner, protocol)),
        TypeRef::Vector { inner } | TypeRef::Set { inner, .. } => {
            format!("c.vector({})", codec_expr(inner, protocol))
        }
        TypeRef::Pair { first, second } => format!(
            "c.pair({}, {})",
            codec_expr(first, protocol),
            codec_expr(second, protocol)
        ),
        TypeRef::Map { key, value, .. } | TypeRef::FlatMap { key, value, .. } => format!(
            "c.vector(c.pair({}, {}))",
            codec_expr(key, protocol),
            codec_expr(value, protocol)
        ),
        TypeRef::Ref { name: n } | TypeRef::StaticVariantRef { name: n } => {
            format!("{}Codec", name(n))
        }
        TypeRef::AnyJson { .. } => "c.unknownValue".into(),
        TypeRef::Unsupported { reason, .. } => format!("c.unsupported({})", q(reason)),
    }
}
fn qualified_ts(ty: &TypeRef) -> String {
    // Prefix only type identifiers, never TS keywords or property names.
    let raw = ts(ty);
    let mut out = String::new();
    let mut word = String::new();
    let flush = |out: &mut String, word: &mut String| {
        if !word.is_empty() {
            if word.chars().next().is_some_and(char::is_uppercase)
                && !["Readonly", "ReadonlyArray", "Record", "Uint8Array"].contains(&word.as_str())
            {
                out.push_str("t.");
            }
            out.push_str(word);
            word.clear();
        }
    };
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            word.push(ch);
        } else {
            flush(&mut out, &mut word);
            out.push(ch);
        }
    }
    flush(&mut out, &mut word);
    out
}

pub fn render(protocol: &Protocol, spec_hash: &str) -> Result<BTreeMap<String, String>> {
    validate(protocol)?;
    let header = format!(
        "// Generated by open-graphene-gen-bindings-ts. Do not edit.\n// Chain: {}; schema: {}; profile: {}; spec SHA-256: {}\n\n",
        protocol.chain.id, protocol.schema_version, PROFILE_VERSION, spec_hash
    );
    let mut files = BTreeMap::new();
    let mut ids = String::from(
        "import { typedId } from '@open-graphene/primitives';\nimport type { Brand } from '@open-graphene/primitives';\nexport type { ObjectId, PublicKey, TimePointSec, VoteId } from '@open-graphene/primitives';\n",
    );
    writeln!(
        ids,
        "export const CHAIN = {} as const;",
        serde_json::to_string(&protocol.chain)?
    )?;
    for object in &protocol.object_types {
        if object.object_type == "vote" {
            continue;
        }
        let n = format!("{}Id", name(&object.object_type));
        writeln!(
            ids,
            "export type {n} = Brand<string, {}>;",
            q(&format!("{}:{n}", protocol.chain.id))
        )?;
        if let (Some(space), Some(ty)) = (object.object_space, object.type_id) {
            writeln!(
                ids,
                "export const {n} = (value: string): {n} => typedId<{}>(value, {space}, {ty});",
                q(&format!("{}:{n}", protocol.chain.id))
            )?;
        }
    }
    files.insert("ids.ts".into(), ids);
    let mut types = String::from(
        "import type { ObjectId, PublicKey, TimePointSec, VoteId } from './ids.js';\nexport type { ObjectId, PublicKey, TimePointSec, VoteId } from './ids.js';\n",
    );
    let id_names = protocol
        .object_types
        .iter()
        .filter(|o| o.object_type != "vote")
        .map(|o| format!("{}Id", name(&o.object_type)))
        .collect::<Vec<_>>()
        .join(", ");
    if !id_names.is_empty() {
        writeln!(
            types,
            "import type {{ {id_names} }} from './ids.js';\nexport type {{ {id_names} }} from './ids.js';"
        )?;
    }
    let mut json_code = String::from(
        "import * as c from '@open-graphene/codec';\nimport type * as t from './types.js';\nexport { parseJson, stringifyJson, CodecError } from '@open-graphene/codec';\n",
    );
    for def in &protocol.enums {
        let n = name(&def.name);
        let entries = def
            .values
            .iter()
            .map(|v| format!("{}: {}", q(&v.name), v.value))
            .collect::<Vec<_>>()
            .join(", ");
        writeln!(types, "export const {n} = {{ {entries} }} as const;")?;
        writeln!(
            types,
            "export type {n} = {};",
            if def.is_bitfield || def.values.is_empty() {
                "number".into()
            } else {
                format!("typeof {n}[keyof typeof {n}]")
            }
        )?;
        let values = def.values.iter().map(|v| v.value).collect::<Vec<_>>();
        writeln!(
            json_code,
            "export const {n}Codec = c.enumNumber({}, {}) as c.Codec<t.{n}>;",
            serde_json::to_string(&values)?,
            def.is_bitfield
        )?;
    }
    let mut structs: BTreeMap<&str, &[FieldDef]> = protocol
        .structs
        .iter()
        .map(|s| (s.name.as_str(), s.fields.as_slice()))
        .collect();
    for op in &protocol.operations {
        structs.insert(&op.name, &op.fields);
    }
    for (source_name, fields) in structs {
        let n = name(source_name);
        writeln!(types, "export interface {n} {{")?;
        writeln!(
            json_code,
            "export const {n}Codec: c.Codec<t.{n}> = c.lazy(() => c.struct<t.{n}>(["
        )?;
        let mut fields = fields.to_vec();
        fields.sort_by_key(|f| f.index);
        for field in &fields {
            let (ty, optional) = match &field.ty {
                TypeRef::Optional { inner } => (inner.as_ref(), true),
                ty => (ty, false),
            };
            writeln!(
                types,
                "  readonly {}{}: {};",
                q(&field.name),
                if optional { "?" } else { "" },
                ts(ty)
            )?;
            let mut expr = codec_expr(ty, protocol);
            if field.name == "extensions" && LEGACY_ACCESS_OPERATIONS.contains(&source_name) {
                expr = "AccessExtensionsCodec".into();
            }
            let default =
                if !optional && DEFAULT_FIELDS.contains(&(source_name, field.name.as_str())) {
                    ", defaultValue: 0"
                } else {
                    ""
                };
            writeln!(
                json_code,
                "  {{ name: {}, codec: {expr}, optional: {optional}{default} }},",
                q(&field.name)
            )?;
        }
        types.push_str("}\n");
        let strict = source_name.ends_with("_ext") || source_name == "data_room_access_extensions";
        writeln!(json_code, "], {{ rejectUnknown: {strict} }}));")?;
    }
    for def in &protocol.static_variants {
        let n = name(&def.name);
        let arms = def
            .variants
            .iter()
            .map(|a| format!("readonly [{}, {}]", a.tag, ts(&a.ty)))
            .collect::<Vec<_>>();
        writeln!(
            types,
            "export type {n} = {};",
            if arms.is_empty() {
                "never".into()
            } else {
                arms.join(" | ")
            }
        )?;
        writeln!(
            json_code,
            "export const {n}Codec: c.Codec<t.{n}> = c.lazy(() => c.variant<t.{n}>({{"
        )?;
        for arm in &def.variants {
            writeln!(
                json_code,
                "  {}: {},",
                arm.tag,
                codec_expr(&arm.ty, protocol)
            )?;
        }
        json_code.push_str("}));\n");
    }
    types.push_str("export type Signature = Uint8Array;\nexport type Config = Readonly<Record<string, unknown>>;\n");
    if protocol.structs.iter().any(|s| s.name == "asset") {
        types.push_str(
            "export type RequiredFee = Asset | readonly [Asset, readonly RequiredFee[]];\n",
        );
        json_code.push_str("export const RequiredFeeCodec = c.requiredFee(AssetCodec) as c.Codec<t.RequiredFee>;\n");
    }
    json_code.push_str(
        "export const ConfigCodec = c.config;\nexport const SignatureCodec = c.bytes(65);\n",
    );
    let mut routes = Vec::new();
    let mut object_arms = Vec::new();
    for object in &protocol.object_types {
        if let (Some(reference), Some(space), Some(ty)) =
            (&object.struct_ref, object.object_space, object.type_id)
        {
            object_arms.push(format!(
                "{{ readonly kind: {}; readonly value: {} }}",
                q(&object.object_type),
                name(reference)
            ));
            routes.push(format!(
                "{}: {{ kind: {}, codec: {}Codec }}",
                q(&format!("{space}.{ty}")),
                q(&object.object_type),
                name(reference)
            ));
        }
    }
    object_arms.push("{ readonly kind: 'unknown'; readonly value: unknown }".into());
    writeln!(
        types,
        "export type ProtocolObject = {};",
        object_arms.join(" | ")
    )?;
    writeln!(
        json_code,
        "export const ProtocolObjectCodec: c.Codec<t.ProtocolObject> = c.lazy(() => c.objectUnion<t.ProtocolObject>({{ {} }}));",
        routes.join(", ")
    )?;
    if protocol
        .structs
        .iter()
        .any(|s| s.name == "data_room_access_extensions")
    {
        json_code.push_str("const AccessExtensionsCodec: c.Codec<t.DataRoomAccessExtensions> = c.struct([{ name: 'expected_access_state', codec: c.text, optional: true }], { legacyEmptyArray: true, rejectUnknown: true });\n");
    }
    files.insert("types.ts".into(), types);
    files.insert("json.ts".into(), json_code);
    let mut operations =
        String::from("import type * as t from './types.js';\nexport const operation = {\n");
    for op in &protocol.operations {
        writeln!(
            operations,
            "  {}: (value: t.{}): readonly [{}, t.{}] => [{}, value],",
            q(op.name.strip_suffix("_operation").unwrap_or(&op.name)),
            name(&op.name),
            op.wire_tag,
            name(&op.name),
            op.wire_tag
        )?;
    }
    operations.push_str("} as const;\n");
    writeln!(
        operations,
        "export const virtualOperationTags: ReadonlySet<number> = new Set({});",
        serde_json::to_string(
            &protocol
                .operations
                .iter()
                .filter(|o| o.is_virtual)
                .map(|o| o.wire_tag)
                .collect::<Vec<_>>()
        )?
    )?;
    files.insert("operations.ts".into(), operations);
    let mut rpc = String::from(
        "import * as c from '@open-graphene/codec';\nimport * as codecs from './json.js';\nimport type * as t from './types.js';\n",
    );
    let mut api_methods: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for method in &protocol.rpc_methods {
        let api = method.api_name.as_deref().unwrap_or(&method.api_class);
        let symbol = format!("{}{}", name(api), name(&method.name));
        writeln!(rpc, "export interface {symbol}Params {{")?;
        let mut params = method.params.clone();
        params.sort_by_key(|p| p.index);
        for p in &params {
            writeln!(
                rpc,
                "  readonly {}{}: {};",
                q(&p.name),
                if p.required { "" } else { "?" },
                qualified_ts(&p.ty)
            )?;
        }
        rpc.push_str("}\n");
        let result = method.returns.as_ref().map_or("void".into(), |t| {
            if matches!(t, TypeRef::Void) {
                "void".into()
            } else {
                qualified_ts(t)
            }
        });
        let result_codec = method.returns.as_ref().map_or("c.rpcVoid".into(), |t| {
            if matches!(t, TypeRef::Void) {
                "c.rpcVoid".into()
            } else {
                rpc_codec_expr(t, protocol)
            }
        });
        writeln!(
            rpc,
            "export const {symbol} = c.rpc<{symbol}Params, {result}>({}, {}, [",
            q(api),
            q(&method.name)
        )?;
        for p in &params {
            writeln!(
                rpc,
                "  {{ name: {}, codec: {}, required: {}, nullable: {} }},",
                q(&p.name),
                rpc_codec_expr(&p.ty, protocol),
                p.required,
                p.nullable
            )?;
        }
        writeln!(rpc, "], {result_codec} as c.Codec<{result}>);")?;
        api_methods
            .entry(api.into())
            .or_default()
            .push(format!("{}: {symbol}", q(&method.name)));
    }
    rpc.push_str("export const rpc = {\n");
    for (api, methods) in api_methods {
        writeln!(rpc, "  {}: {{ {} }},", q(&api), methods.join(", "))?;
    }
    rpc.push_str("} as const;\n");
    files.insert("rpc.ts".into(), rpc);
    files.insert("fc.ts".into(), fc::render(protocol)?);
    files.insert("index.ts".into(), "export * from './ids.js';\nexport * from './types.js';\nexport * from './json.js';\nexport * from './operations.js';\nexport * from './rpc.js';\nexport * from './fc.js';\n".into());
    for content in files.values_mut() {
        *content = format!("{header}{content}");
    }
    let mut dynamic = Vec::new();
    collect_dynamic(&serde_json::to_value(protocol)?, "", &mut dynamic);
    files.insert("support.json".into(), serde_json::to_string_pretty(&json!({
        "chain": protocol.chain.id, "schemaVersion": protocol.schema_version, "profileVersion": PROFILE_VERSION, "specSha256": spec_hash,
        "stage": "transfer-fc", "dynamicFields": dynamic,
        "operations": protocol.operations.iter().map(|op| json!({"name":op.name, "tag":op.wire_tag, "virtual":op.is_virtual, "type":true, "json":"generated", "fc": if op.name == "transfer_operation" { "empty_extensions_only" } else { "not_implemented" }, "signing": if op.name == "transfer_operation" && protocol.chain.id == "swaplock" { "swaplock-low-s" } else { "not_implemented" }, "builder":op.name == "transfer_operation" && protocol.chain.id == "swaplock"})).collect::<Vec<_>>(),
        "rpc": protocol.rpc_methods.iter().map(|m| json!({"api":m.api_name,"method":m.name,"descriptor":true,"highLevel":false,"subscription":false})).collect::<Vec<_>>()
    }))? + "\n");
    Ok(files)
}
fn rpc_codec_expr(ty: &TypeRef, protocol: &Protocol) -> String {
    let expr = codec_expr(ty, protocol);
    let mut out = String::new();
    let mut word = String::new();
    let flush = |out: &mut String, word: &mut String| {
        if !word.is_empty() {
            if word.ends_with("Codec") {
                out.push_str("codecs.");
            }
            out.push_str(word);
            word.clear();
        }
    };
    for ch in expr.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            word.push(ch);
        } else {
            flush(&mut out, &mut word);
            out.push(ch);
        }
    }
    flush(&mut out, &mut word);
    out
}
fn collect_dynamic(value: &Value, path: &str, result: &mut Vec<Value>) {
    match value {
        Value::Object(map) => {
            if matches!(
                map.get("kind").and_then(Value::as_str),
                Some("any_json" | "unsupported")
            ) {
                result.push(json!({"path":path,"reason":map.get("reason")}));
            }
            for (k, v) in map {
                collect_dynamic(v, &format!("{path}/{k}"), result);
            }
        }
        Value::Array(items) => {
            for (i, v) in items.iter().enumerate() {
                collect_dynamic(v, &format!("{path}/{i}"), result);
            }
        }
        _ => {}
    }
}

pub fn generate(spec: &Path, output: &Path, check: bool) -> Result<()> {
    let source = fs::read(spec)?;
    let protocol: Protocol = serde_json::from_slice(&source)?;
    let files = render(&protocol, &format!("{:x}", Sha256::digest(&source)))?;
    let expected: BTreeSet<_> = files.keys().cloned().collect();
    let actual = if output.exists() {
        fs::read_dir(output)?
            .map(|entry| entry.map(|e| e.file_name().to_string_lossy().into_owned()))
            .collect::<std::io::Result<BTreeSet<_>>>()?
    } else {
        BTreeSet::new()
    };
    if check {
        if actual != expected {
            return Err(format!("generated file set drifted in {}", output.display()).into());
        }
        for (name, contents) in &files {
            if fs::read_to_string(output.join(name))? != *contents {
                return Err(format!("generated output drifted: {name}").into());
            }
        }
        println!("checked {} ({} files)", protocol.chain.id, files.len());
        return Ok(());
    }
    // Refuse to delete files not positively identified as this generator's own.
    for stale in actual.difference(&expected) {
        let path = output.join(stale);
        if !fs::read_to_string(&path)?.starts_with("// Generated by open-graphene-gen-bindings-ts.")
        {
            return Err(format!("refusing to replace unmanaged file {}", path.display()).into());
        }
    }
    fs::create_dir_all(output)?;
    for (name, contents) in files {
        fs::write(output.join(name), contents)?;
    }
    for stale in actual.difference(&expected) {
        fs::remove_file(output.join(stale))?;
    }
    println!("generated {} bindings", protocol.chain.id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);

    fn fixture() -> Protocol {
        serde_json::from_str(include_str!("../../../../open-graphene-packages/rust/graphene-chain-swaplock/graphene-chain-swaplock-spec/dist/swaplock.open-graphene.json")).unwrap()
    }
    #[test]
    fn rejects_ambiguous_or_missing_protocol_definitions() {
        let original = fixture();
        let mut p = original.clone();
        p.schema_version = 999;
        assert!(
            validate(&p)
                .unwrap_err()
                .to_string()
                .contains("schemaVersion")
        );
        let mut p = original.clone();
        p.operations[1].wire_tag = p.operations[0].wire_tag;
        assert!(
            validate(&p)
                .unwrap_err()
                .to_string()
                .contains("duplicate operation tag")
        );
        let mut p = original.clone();
        p.structs[0].fields[0].index = 99;
        assert!(validate(&p).is_err());
        let mut p = original.clone();
        p.structs[0].fields[0].ty = TypeRef::Ref {
            name: "does_not_exist".into(),
        };
        assert!(validate(&p).is_err());
        let mut p = original;
        p.structs.push(p.structs[0].clone());
        assert!(validate(&p).is_err());
    }
    #[test]
    fn deterministic_output_and_check_detect_content_and_file_set_drift() {
        let p = fixture();
        assert_eq!(render(&p, "test").unwrap(), render(&p, "test").unwrap());
        let temp = std::env::temp_dir().join(format!(
            "graphene-ts-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&temp).unwrap();
        let spec = temp.join("spec.json");
        let out = temp.join("generated");
        fs::write(&spec, serde_json::to_vec(&p).unwrap()).unwrap();
        generate(&spec, &out, false).unwrap();
        generate(&spec, &out, true).unwrap();
        fs::write(out.join("types.ts"), "drift").unwrap();
        assert!(generate(&spec, &out, true).is_err());
        generate(&spec, &out, false).unwrap();
        fs::write(out.join("unexpected.txt"), "user owned").unwrap();
        assert!(generate(&spec, &out, true).is_err());
        assert!(generate(&spec, &out, false).is_err());
        assert_eq!(
            fs::read_to_string(out.join("unexpected.txt")).unwrap(),
            "user owned"
        );
        fs::remove_dir_all(temp).unwrap();
    }
    #[test]
    fn invalid_spec_does_not_modify_existing_output() {
        let temp = std::env::temp_dir().join(format!(
            "graphene-ts-invalid-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&temp).unwrap();
        let spec = temp.join("spec.json");
        let out = temp.join("generated");
        let mut p = fixture();
        fs::write(&spec, serde_json::to_vec(&p).unwrap()).unwrap();
        generate(&spec, &out, false).unwrap();
        let before = fs::read(out.join("types.ts")).unwrap();
        p.schema_version = 999;
        fs::write(&spec, serde_json::to_vec(&p).unwrap()).unwrap();
        assert!(generate(&spec, &out, false).is_err());
        assert_eq!(fs::read(out.join("types.ts")).unwrap(), before);
        fs::remove_dir_all(temp).unwrap();
    }
}
