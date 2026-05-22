use std::path::PathBuf;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SourceFacts {
    pub fc_apis: Vec<FcApi>,
    pub classes: Vec<RawClass>,
    pub enums: Vec<RawEnum>,
    pub static_variants: Vec<RawStaticVariant>,
    pub object_types: Vec<RawObjectType>,
    pub reflects: Vec<RawReflect>,
    pub diagnostics: Vec<ExtractDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FcApi {
    pub api_class: String,
    pub method_names: Vec<String>,
    pub source: SourceLoc,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawClass {
    pub name: String,
    pub qualified_name: Option<String>,
    pub methods: Vec<RawMethod>,
    pub fields: Vec<RawField>,
    pub source: SourceLoc,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawField {
    pub name: String,
    pub type_expr: String,
    pub source: SourceLoc,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawEnum {
    pub name: String,
    pub values: Vec<RawEnumValue>,
    pub source: SourceLoc,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawEnumValue {
    pub name: String,
    pub value: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawStaticVariant {
    pub name: String,
    pub variants: Vec<String>,
    pub source: SourceLoc,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawObjectType {
    pub object_type: String,
    pub cpp_alias: String,
    pub object_space_name: String,
    pub object_space: Option<u32>,
    pub object_type_name: String,
    pub type_id: Option<u32>,
    pub struct_ref: Option<String>,
    pub source: SourceLoc,
    pub id_namespace: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawReflect {
    pub type_name: String,
    pub bases: Vec<String>,
    pub fields: Vec<String>,
    pub source: SourceLoc,
    pub derived: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawMethod {
    pub name: String,
    pub return_type: String,
    pub params: Vec<RawParam>,
    pub is_const: bool,
    pub source: SourceLoc,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawParam {
    pub name: String,
    pub type_expr: String,
    pub default_value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLoc {
    pub file: PathBuf,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractDiagnostic {
    pub code: String,
    pub message: String,
    pub source: Option<SourceLoc>,
}
