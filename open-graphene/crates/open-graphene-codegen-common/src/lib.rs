//! Audited compatibility rules not yet expressible in the source-derived IR.
//! Shared by language generators; no language syntax or SDK policy lives here.
pub const PROFILE_VERSION: u32 = 1;

/// Empty access guards were historically `extensions_type` arrays on JSON wire.
/// Source: Swaplock data_room.hpp; Rust access-precondition C++ wire fixtures.
pub const LEGACY_ACCESS_OPERATIONS: &[&str] = &[
    "data_room_member_add_operation",
    "data_room_member_update_operation",
    "data_room_member_remove_operation",
    "data_room_rotate_key_operation",
];

/// Missing write_policy means legacy (0), never strict. Other entries are
/// optionals introduced after deployment and default to absent, not a payload.
pub const DEFAULT_FIELDS: &[(&str, &str)] = &[
    ("data_room_object", "write_policy"),
    ("data_room_create_operation_ext", "write_policy"),
    ("content_card_remove_operation_ext", "expected_hash"),
    ("content_card_update_operation_ext", "expected_hash"),
];

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
