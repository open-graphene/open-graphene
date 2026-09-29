//! FC schemas and checked entry points generated from the shared protocol IR.
use super::{Result, name, q};
use open_graphene_json_schema::Protocol;
use serde_json::json;
use std::{collections::BTreeMap, fmt::Write};

pub fn render(protocol: &Protocol) -> Result<String> {
    let extensions = open_graphene_codegen_common::extension_struct_names(protocol);
    let mut definitions = BTreeMap::new();
    let mut types = BTreeMap::new();
    for s in &protocol.structs {
        let mut fields = s.fields.clone();
        fields.sort_by_key(|f| f.index);
        definitions.insert(s.name.clone(), json!({"kind":"struct", "fields":fields.iter().map(|f| json!({"name":f.name,"type":f.ty,"index":f.index})).collect::<Vec<_>>(), "extension":extensions.contains(&s.name)}));
        types.insert(s.name.clone(), name(&s.name));
    }
    for op in &protocol.operations {
        let mut fields = op.fields.clone();
        fields.sort_by_key(|f| f.index);
        definitions.insert(op.name.clone(), json!({"kind":"struct","fields":fields.iter().map(|f| json!({"name":f.name,"type":f.ty,"index":f.index})).collect::<Vec<_>>()}));
        types.insert(op.name.clone(), name(&op.name));
    }
    for v in &protocol.static_variants {
        definitions.insert(v.name.clone(), json!({"kind":"variant", "arms":v.variants.iter().map(|a| json!({"tag":a.tag,"type":a.ty,"virtual":protocol.operations.iter().any(|op|op.name==a.name&&op.is_virtual)})).collect::<Vec<_>>()}));
        types.insert(v.name.clone(), name(&v.name));
    }
    let objects: BTreeMap<_, _> = protocol
        .object_types
        .iter()
        .filter_map(|o| Some((o.object_type.clone(), [o.object_space?, o.type_id?])))
        .collect();
    let schema = json!({"definitions":definitions,"objects":objects,"prefix":protocol.chain.public_key_prefix});
    let mut out = "import { encodeFc, type FcSchema } from '@open-graphene/fc';\nimport * as codecs from './json.js';\nimport type * as t from './types.js';\n".to_string();
    writeln!(
        out,
        "const schema: FcSchema = {};",
        serde_json::to_string(&schema)?
    )?;
    for (source, n) in types {
        writeln!(
            out,
            "export function encode{n}(input: t.{n}): Uint8Array {{\n  const value = codecs.{n}Codec.decode(codecs.{n}Codec.encode(input));\n  return encodeFc(schema, {}, value);\n}}",
            q(&source)
        )?;
    }
    Ok(out)
}
