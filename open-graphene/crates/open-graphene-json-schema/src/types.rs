use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::support::{SourceMeta, SupportDef};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JsonShape {
    Number,
    DecimalString,
    String,
    Object,
    Array,
    TaggedTuple,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FcEncoding {
    FixedLeI32,
    FixedLeI64,
    FixedLeU64,
    FixedLeU32,
    UnsignedVarint,
    LengthPrefixedUtf8,
    LengthPrefixedBytes,
    FixedBytes,
    PublicKey33WithChecksumInput,
    Signature65,
    StructFields,
    SparseStructFields,
    StaticVariant,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OrderingRule {
    Numeric,
    LexicographicString,
    ProtocolObjectInstance,
    RawBytesLexicographic,
    StaticVariantTag,
    Unresolved,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IntType {
    U8,
    U16,
    U32,
    U64,
    I32,
    I64,
}

/// Canonical v3 type reference.
///
/// The JSON representation intentionally uses the same `kind` tag as the
/// hand-written v3 fixtures in `open-graphene-v5/fixtures/json`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TypeRef {
    /// A method or expression that intentionally returns no JSON value.
    Void,
    Bool,
    Uint8,
    Uint16,
    Uint32,
    Int32 {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        fc: Option<FcEncoding>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        source: Option<SourceMeta>,
    },
    Int64 {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        json: Option<JsonShape>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        fc: Option<FcEncoding>,
    },
    Uint64 {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        json: Option<JsonShape>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        fc: Option<FcEncoding>,
    },
    UnsignedVarint,
    /// Numeric handle used on the wire to correlate RPC notices with a client-side callback.
    CallbackHandle,
    String,
    Bytes,
    FixedHex {
        bytes: u32,
    },
    FixedBytes {
        bytes: u32,
    },
    TimePointSec,
    TimePoint,
    PublicKey {
        #[serde(
            rename = "chainPrefix",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        chain_prefix: Option<String>,
        #[serde(rename = "prefixRef", default, skip_serializing_if = "Option::is_none")]
        prefix_ref: Option<String>,
    },
    Address,
    Signature,
    ObjectId,
    ProtocolObjectId {
        #[serde(rename = "objectType")]
        object_type: String,
    },
    /// Any known protocol object serialized as JSON.
    ///
    /// Generators can expand this to a union/oneOf over [`crate::defs::ObjectTypeDef`]
    /// entries that have a `structRef`.
    ProtocolObjectUnion {
        #[serde(rename = "objectTypes", default, skip_serializing_if = "Vec::is_empty")]
        object_types: Vec<String>,
    },
    VoteId,
    Optional {
        inner: Box<TypeRef>,
    },
    Vector {
        inner: Box<TypeRef>,
    },
    Set {
        inner: Box<TypeRef>,
        ordering: OrderingRule,
    },
    Map {
        key: Box<TypeRef>,
        value: Box<TypeRef>,
        ordering: OrderingRule,
    },
    FlatMap {
        key: Box<TypeRef>,
        value: Box<TypeRef>,
        ordering: OrderingRule,
    },
    Pair {
        first: Box<TypeRef>,
        second: Box<TypeRef>,
    },
    Ref {
        name: String,
    },
    StaticVariantRef {
        name: String,
    },
    #[serde(rename_all = "camelCase")]
    AnyJson {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        source: Option<SourceMeta>,
    },
    #[serde(rename_all = "camelCase")]
    Unsupported {
        reason: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        source: Option<SourceMeta>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        support: Option<SupportDef>,
    },
}
