pub mod classes;
pub mod enums;
pub mod facts;
pub(crate) mod lexer;
pub mod macros;
pub mod object_types;
pub mod reflect;
pub mod static_variants;
pub mod virtual_ops;

use std::fs;

use crate::error::{Result, SpecGenError};
use crate::source::{SourceFileKind, SourceSet};

pub use classes::extract_classes;
pub use enums::{extract_enum_definitions, extract_enums};
pub use facts::{
    ExtractDiagnostic, FcApi, RawClass, RawEnum, RawEnumDefinition, RawEnumMember, RawEnumValue,
    RawField, RawMethod, RawObjectType, RawParam, RawReflect, RawStaticVariant, SourceFacts,
    SourceLoc,
};
pub use macros::extract_fc_apis;
pub use object_types::extract_object_types;
pub use reflect::extract_reflects;
pub use static_variants::extract_static_variants;
pub use virtual_ops::extract_virtual_operation_markers;

pub fn extract_source_facts(source_set: &SourceSet) -> Result<SourceFacts> {
    let mut facts = SourceFacts::default();

    for source_file in &source_set.files {
        let text = fs::read_to_string(&source_file.path).map_err(|source| {
            SpecGenError::ReadSourceFile {
                path: source_file.path.clone(),
                source,
            }
        })?;
        // Recorded source locations must be machine-independent: strip the
        // chain-repo prefix so the emitted spec is reproducible anywhere.
        let loc_path = source_file
            .path
            .strip_prefix(&source_set.chain_repo)
            .unwrap_or(&source_file.path);
        // Translation units are scanned for reflect macros only: their local
        // classes and helpers are implementation detail, not protocol surface.
        if source_file.kind == SourceFileKind::Source {
            facts.reflects.extend(extract_reflects(&text, loc_path));
            continue;
        }
        facts.fc_apis.extend(extract_fc_apis(&text, loc_path));
        facts.classes.extend(extract_classes(&text, loc_path));
        facts.enums.extend(extract_enums(&text, loc_path));
        facts
            .enum_definitions
            .extend(extract_enum_definitions(&text, loc_path));
        facts
            .static_variants
            .extend(extract_static_variants(&text, loc_path));
        facts
            .object_types
            .extend(extract_object_types(&text, loc_path));
        facts.reflects.extend(extract_reflects(&text, loc_path));
        facts
            .virtual_operations
            .extend(extract_virtual_operation_markers(&text));
    }

    // The substring-based static-variant scan can desync on `using namespace`
    // lines and produce garbage names; keep the record out of the facts but
    // never drop it silently.
    facts.static_variants.retain(|variant| {
        let valid = is_valid_cpp_identifier(&variant.name);
        if !valid {
            facts.diagnostics.push(ExtractDiagnostic {
                code: "static-variant-name-unparseable".to_string(),
                message: format!(
                    "discarded static variant with unparseable name {:?}",
                    truncate_for_message(&variant.name)
                ),
                source: Some(variant.source.clone()),
            });
        }
        valid
    });

    Ok(facts)
}

fn is_valid_cpp_identifier(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|ch| ch.is_alphanumeric() || ch == '_' || ch == ':')
}

fn truncate_for_message(value: &str) -> String {
    const LIMIT: usize = 60;
    if value.chars().count() <= LIMIT {
        value.to_string()
    } else {
        let prefix: String = value.chars().take(LIMIT).collect();
        format!("{prefix}…")
    }
}
