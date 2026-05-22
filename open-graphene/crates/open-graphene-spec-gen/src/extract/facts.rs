use std::path::PathBuf;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SourceFacts {
    pub fc_apis: Vec<FcApi>,
    pub diagnostics: Vec<ExtractDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FcApi {
    pub api_class: String,
    pub method_names: Vec<String>,
    pub source: SourceLoc,
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
