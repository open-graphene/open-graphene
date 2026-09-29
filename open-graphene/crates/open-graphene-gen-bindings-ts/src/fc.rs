//! First audited FC dependency closure: transfer, memo wire data and transactions.
//! Field order, ID spaces, widths and operation tags come from IR. Other
//! operations/containers fail closed until their ordering/support is audited.
use super::{Result, name, q};
use open_graphene_json_schema::{Protocol, TypeRef};
use std::fmt::Write;

const STRUCTS: &[&str] = &[
    "asset",
    "memo_data",
    "transfer_operation",
    "transaction",
    "signed_transaction",
];

fn emit(ty: &TypeRef, value: &str, protocol: &Protocol, depth: usize) -> Result<String> {
    Ok(match ty {
        TypeRef::Uint8 => format!("w.uint({value}, 1);"),
        TypeRef::Uint16 => format!("w.uint({value}, 2);"),
        TypeRef::Uint32 => format!("w.uint({value}, 4);"),
        TypeRef::Int64 { .. } => format!("w.int({value}, 8);"),
        TypeRef::Uint64 { .. } => format!("w.uint({value}, 8);"),
        TypeRef::TimePointSec => format!("w.time({value});"),
        TypeRef::Bytes => format!("w.varint({value}.length); w.bytes({value});"),
        TypeRef::Signature => format!("w.bytes({value}, 65);"),
        TypeRef::PublicKey { chain_prefix, .. } => format!(
            "w.bytes(decodePublicKey({value}, {}), 33);",
            q(chain_prefix
                .as_deref()
                .unwrap_or(&protocol.chain.public_key_prefix))
        ),
        TypeRef::ProtocolObjectId { object_type } => {
            let object = protocol
                .object_types
                .iter()
                .find(|o| o.object_type == *object_type)
                .ok_or("missing FC object type")?;
            format!(
                "w.typedId({value}, {}, {});",
                object.object_space.ok_or("unresolved FC object space")?,
                object.type_id.ok_or("unresolved FC object ID")?
            )
        }
        TypeRef::Ref { name: reference } if STRUCTS.contains(&reference.as_str()) => {
            format!("write{}(w, {value});", name(reference))
        }
        TypeRef::StaticVariantRef { name: reference } if reference == "operation" => {
            format!("writeOperation(w, {value});")
        }
        TypeRef::Optional { inner } => format!(
            "if ({value} == null) {{ w.uint(0, 1); }} else {{ w.uint(1, 1); {} }}",
            emit(inner, value, protocol, depth + 1)?
        ),
        TypeRef::Vector { inner } => {
            let item = format!("item{depth}");
            format!(
                "w.varint({value}.length); for (const {item} of {value}) {{ {} }}",
                emit(inner, &item, protocol, depth + 1)?
            )
        }
        TypeRef::Set { inner, .. } if matches!(inner.as_ref(), TypeRef::StaticVariantRef { name } if name == "future_extensions") =>
        {
            format!(
                "if ({value}.length !== 0) throw new Error('Nonempty future extensions are not supported for signing'); w.varint(0);"
            )
        }
        _ => return Err(format!("unaudited type in transfer FC closure: {ty:?}").into()),
    })
}

pub fn render(protocol: &Protocol) -> Result<String> {
    let mut out = "import { FcWriter, decodePublicKey } from '@open-graphene/fc';\nimport * as codecs from './json.js';\nimport type * as t from './types.js';\n".to_string();
    let transfer = protocol
        .operations
        .iter()
        .find(|op| op.name == "transfer_operation" && !op.is_virtual)
        .ok_or("missing nonvirtual transfer operation")?;
    writeln!(
        out,
        "function writeOperation(w: FcWriter, value: t.Operation): void {{\n  if (value[0] !== {}) throw new Error('FC signing currently supports transfer only');\n  w.varint(value[0]); writeTransferOperation(w, value[1]);\n}}",
        transfer.wire_tag
    )?;
    for source in STRUCTS {
        let def = protocol
            .structs
            .iter()
            .find(|s| s.name == *source)
            .ok_or_else(|| format!("missing FC root {source}"))?;
        let n = name(source);
        writeln!(out, "function write{n}(w: FcWriter, value: t.{n}): void {{")?;
        let mut fields = def.fields.iter().collect::<Vec<_>>();
        fields.sort_by_key(|f| f.index);
        for field in fields {
            writeln!(
                out,
                "  {}",
                emit(
                    &field.ty,
                    &format!("value[{}]", q(&field.name)),
                    protocol,
                    0
                )?
            )?;
        }
        out.push_str("}\n");
        writeln!(
            out,
            "export function encode{n}(input: t.{n}): Uint8Array {{\n  const value = codecs.{n}Codec.decode(codecs.{n}Codec.encode(input));\n  const writer = new FcWriter(); write{n}(writer, value); return writer.finish();\n}}"
        )?;
    }
    Ok(out)
}
