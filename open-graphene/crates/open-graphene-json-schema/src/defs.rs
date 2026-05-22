use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::support::{SourceMeta, SupportDef};
use crate::types::{IntType, TypeRef};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChainDef {
    pub id: String,
    pub public_key_prefix: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StructKind {
    Struct,
    SparseStruct,
    Operation,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FieldDef {
    pub index: u32,
    pub name: String,
    #[serde(rename = "type")]
    pub ty: TypeRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<SourceMeta>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub support: Option<SupportDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StructDef {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_name: Option<String>,
    pub kind: StructKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wire_tag: Option<u32>,
    #[serde(default)]
    pub fields: Vec<FieldDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub support: Option<SupportDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EnumDef {
    pub name: String,
    pub underlying: IntType,
    #[serde(default)]
    pub values: Vec<EnumValueDef>,
    #[serde(default)]
    pub is_bitfield: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<SourceMeta>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub support: Option<SupportDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EnumValueDef {
    pub name: String,
    pub value: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StaticVariantArmDef {
    pub tag: u32,
    pub name: String,
    #[serde(rename = "type")]
    pub ty: TypeRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub support: Option<SupportDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StaticVariantDef {
    pub name: String,
    pub kind: String,
    pub json: String,
    pub fc: String,
    #[serde(default)]
    pub variants: Vec<StaticVariantArmDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<SourceMeta>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub support: Option<SupportDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OperationDef {
    pub name: String,
    pub wire_tag: u32,
    #[serde(default)]
    pub fields: Vec<FieldDef>,
    #[serde(default)]
    pub is_virtual: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<SourceMeta>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub support: Option<SupportDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ObjectTypeDef {
    pub object_type: String,
    pub cpp_alias: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object_space: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub type_id: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub struct_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<SourceMeta>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub support: Option<SupportDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RpcMethodDef {
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub api_class: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_name: Option<String>,
    #[serde(default)]
    pub params: Vec<FieldDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub returns: Option<TypeRef>,
    #[serde(default)]
    pub is_subscription: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub binding_hints: Option<RpcBindingHintsDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<SourceMeta>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub support: Option<SupportDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RpcBindingHintsDef {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub callback_param_index: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dependent_return: Option<DependentReturnDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DependentReturnDef {
    #[serde(rename_all = "camelCase")]
    ObjectById { id_param_index: u32 },
    #[serde(rename_all = "camelCase")]
    ObjectByIdVector { id_param_index: u32 },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StrictModeDef {
    pub unknown_types: String,
    pub duplicate_field_indexes: String,
    pub duplicate_variant_tags: String,
    pub unsupported_signing_shape: String,
}
