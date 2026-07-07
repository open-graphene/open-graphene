pub mod classes;
pub mod enums;
pub mod facts;
pub mod macros;
pub mod object_types;
pub mod reflect;
pub mod static_variants;
pub mod virtual_ops;

use std::fs;

use crate::error::{Result, SpecGenError};
use crate::source::SourceSet;

pub use classes::extract_classes;
pub use enums::extract_enums;
pub use facts::{
    ExtractDiagnostic, FcApi, RawClass, RawEnum, RawEnumValue, RawField, RawMethod, RawObjectType,
    RawParam, RawReflect, RawStaticVariant, SourceFacts, SourceLoc,
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
        facts.fc_apis.extend(extract_fc_apis(&text, loc_path));
        facts.classes.extend(extract_classes(&text, loc_path));
        facts.enums.extend(extract_enums(&text, loc_path));
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

    Ok(facts)
}
