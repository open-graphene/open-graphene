//! Audited compatibility rules not yet expressible in the source-derived IR.
//! Shared by language generators; no language syntax or SDK policy lives here.
pub const PROFILE_VERSION: u32 = 1;

pub const RPC_SYNTHETIC_REFS: &[&str] = &["config", "required_fee"];

pub fn extension_struct_names(
    protocol: &open_graphene_json_schema::Protocol,
) -> std::collections::BTreeSet<String> {
    use open_graphene_json_schema::TypeRef;
    protocol
        .structs
        .iter()
        .flat_map(|s| &s.fields)
        .chain(protocol.operations.iter().flat_map(|s| &s.fields))
        .filter_map(|f| match &f.ty {
            TypeRef::Ref { name } if f.name == "extensions" => Some(name.clone()),
            _ => None,
        })
        .collect()
}
