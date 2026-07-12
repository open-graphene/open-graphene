//! Shared protocol fixtures for the generator's unit tests.

use open_graphene_json_schema::{ChainDef, FieldDef, Protocol, TypeRef};

pub(crate) fn transfer_operation_fields() -> Vec<FieldDef> {
    let field = |index: u32, name: &str, ty: TypeRef| FieldDef {
        index,
        name: name.to_string(),
        ty,
        source: None,
        support: None,
    };
    vec![
        field(
            0,
            "fee",
            TypeRef::Ref {
                name: "asset".to_string(),
            },
        ),
        field(
            1,
            "from",
            TypeRef::ProtocolObjectId {
                object_type: "account".to_string(),
            },
        ),
        field(
            2,
            "to",
            TypeRef::ProtocolObjectId {
                object_type: "account".to_string(),
            },
        ),
        field(
            3,
            "amount",
            TypeRef::Ref {
                name: "asset".to_string(),
            },
        ),
        field(
            4,
            "memo",
            TypeRef::Optional {
                inner: Box::new(TypeRef::Ref {
                    name: "memo_data".to_string(),
                }),
            },
        ),
        field(
            5,
            "extensions",
            TypeRef::Set {
                inner: Box::new(TypeRef::StaticVariantRef {
                    name: "future_extensions".to_string(),
                }),
                ordering: open_graphene_json_schema::types::OrderingRule::StaticVariantTag,
            },
        ),
    ]
}

pub(crate) fn minimal_protocol() -> Protocol {
    Protocol {
        schema_version: 1,
        chain: ChainDef {
            id: "swaplock".to_string(),
            public_key_prefix: "BTS".to_string(),
            chain_id: None,
        },
        structs: vec![],
        enums: vec![],
        static_variants: vec![],
        operations: vec![],
        object_types: vec![open_graphene_json_schema::ObjectTypeDef {
            object_type: "vote".to_string(),
            cpp_alias: "vote_id_type".to_string(),
            object_space: None,
            type_id: None,
            struct_ref: None,
            source: None,
            support: None,
        }],
        rpc_apis: vec![],
        rpc_methods: vec![],
        strict_mode: None,
    }
}
