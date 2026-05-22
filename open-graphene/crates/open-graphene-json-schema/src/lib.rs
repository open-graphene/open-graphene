//! Canonical v3 IR and JSON Schema for Open Graphene protocol descriptions.
//!
//! This crate intentionally keeps the familiar `open-graphene-json-schema`
//! package name while changing the model toward the v3 contract:
//!
//! - field order is explicit via `index`,
//! - chain metadata includes the public-key prefix,
//! - 64-bit JSON shape and FC encoding are explicit,
//! - container ordering is explicit,
//! - support/deferred/unsupported status carries a reason,
//! - static variants and operation tags are explicit.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub mod defs;
pub mod support;
pub mod types;

pub use defs::{
    ChainDef, EnumDef, EnumValueDef, FieldDef, ObjectTypeDef, OperationDef, RpcMethodDef,
    StaticVariantArmDef, StaticVariantDef, StrictModeDef, StructDef, StructKind,
};
pub use support::{SourceMeta, SupportDef, SupportStatus};
pub use types::{FcEncoding, JsonShape, OrderingRule, TypeRef};

/// Root canonical IR document for one Graphene-family protocol.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Protocol {
    pub schema_version: u32,
    pub chain: ChainDef,
    #[serde(default)]
    pub structs: Vec<StructDef>,
    #[serde(default)]
    pub enums: Vec<EnumDef>,
    #[serde(default)]
    pub static_variants: Vec<StaticVariantDef>,
    #[serde(default)]
    pub operations: Vec<OperationDef>,
    #[serde(default)]
    pub object_types: Vec<ObjectTypeDef>,
    #[serde(default)]
    pub rpc_methods: Vec<RpcMethodDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strict_mode: Option<StrictModeDef>,
}

/// Emit the JSON Schema for [`Protocol`] as pretty JSON.
pub fn emit_schema() -> String {
    let schema = schemars::schema_for!(Protocol);
    serde_json::to_string_pretty(&schema).expect("schemars output is valid JSON")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_build_empty_protocol() {
        let protocol = Protocol {
            schema_version: 1,
            chain: ChainDef {
                id: "swaplock-minimal".to_string(),
                public_key_prefix: "SWP".to_string(),
            },
            structs: vec![],
            enums: vec![],
            static_variants: vec![],
            operations: vec![],
            object_types: vec![],
            rpc_methods: vec![],
            strict_mode: None,
        };

        let json = serde_json::to_value(&protocol).expect("serialize protocol");
        assert_eq!(json["schemaVersion"], 1);
        assert_eq!(json["chain"]["publicKeyPrefix"], "SWP");
    }

    #[test]
    fn emits_schema_json() {
        let schema = emit_schema();
        let json: serde_json::Value = serde_json::from_str(&schema).expect("schema is valid JSON");
        assert!(json.is_object());
    }

    #[test]
    fn type_ref_json_matches_v3_fixture_shape() {
        let int64 = serde_json::to_value(TypeRef::Int64 {
            json: Some(JsonShape::DecimalString),
            fc: Some(FcEncoding::FixedLeI64),
        })
        .expect("serialize int64");
        assert_eq!(int64["kind"], "int64");
        assert_eq!(int64["json"], "decimal_string");
        assert_eq!(int64["fc"], "fixed_le_i64");

        let account_id = serde_json::to_value(TypeRef::ProtocolObjectId {
            object_type: "account".to_string(),
        })
        .expect("serialize protocol object id");
        assert_eq!(account_id["kind"], "protocol_object_id");
        assert_eq!(account_id["objectType"], "account");

        let public_key = serde_json::to_value(TypeRef::PublicKey {
            chain_prefix: Some("SWP".to_string()),
            prefix_ref: None,
        })
        .expect("serialize public key");
        assert_eq!(public_key["kind"], "public_key");
        assert_eq!(public_key["chainPrefix"], "SWP");
    }

    #[test]
    fn missing_object_types_defaults_to_empty_vec() {
        let json = serde_json::json!({
            "schemaVersion": 1,
            "chain": {
                "id": "swaplock-minimal",
                "publicKeyPrefix": "SWP"
            },
            "structs": [],
            "enums": [],
            "staticVariants": [],
            "operations": [],
            "rpcMethods": []
        });

        let protocol: Protocol =
            serde_json::from_value(json).expect("deserialize old protocol JSON");
        assert!(protocol.object_types.is_empty());
    }

    #[test]
    fn object_and_rpc_metadata_serialize_as_camel_case() {
        let object_type = ObjectTypeDef {
            object_type: "account".to_string(),
            cpp_alias: "account_id_type".to_string(),
            object_space: Some(1),
            type_id: Some(2),
            source: Some(SourceMeta {
                name: Some("object_id<1,2>".to_string()),
                legacy_name: None,
                file: Some("protocol/types.hpp".to_string()),
                line: Some(42),
            }),
            support: None,
        };
        let method = RpcMethodDef {
            name: "get_objects".to_string(),
            api_class: "database_api".to_string(),
            api_name: Some("database".to_string()),
            params: vec![],
            returns: Some(TypeRef::Vector {
                inner: Box::new(TypeRef::ObjectId),
            }),
            is_subscription: false,
            source: None,
            support: None,
        };

        let protocol = Protocol {
            schema_version: 1,
            chain: ChainDef {
                id: "swaplock-minimal".to_string(),
                public_key_prefix: "SWP".to_string(),
            },
            structs: vec![],
            enums: vec![],
            static_variants: vec![],
            operations: vec![],
            object_types: vec![object_type],
            rpc_methods: vec![method],
            strict_mode: None,
        };

        let json = serde_json::to_value(&protocol).expect("serialize protocol metadata");
        assert_eq!(json["objectTypes"][0]["objectType"], "account");
        assert_eq!(json["objectTypes"][0]["cppAlias"], "account_id_type");
        assert_eq!(json["objectTypes"][0]["objectSpace"], 1);
        assert_eq!(json["objectTypes"][0]["typeId"], 2);
        assert_eq!(json["objectTypes"][0]["source"]["line"], 42);
        assert_eq!(json["rpcMethods"][0]["apiClass"], "database_api");
        assert_eq!(json["rpcMethods"][0]["apiName"], "database");
    }

    #[test]
    fn old_rpc_method_json_defaults_missing_api_identity() {
        let json = serde_json::json!({
            "name": "get_objects",
            "params": [],
            "isSubscription": false
        });

        let method: RpcMethodDef =
            serde_json::from_value(json).expect("deserialize old RPC method");
        assert_eq!(method.api_class, "");
        assert_eq!(method.api_name, None);
    }
}
