pub mod classes;
pub mod enums;
pub mod facts;
pub mod macros;
pub mod object_types;
pub mod reflect;
pub mod static_variants;

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

pub fn extract_source_facts(source_set: &SourceSet) -> Result<SourceFacts> {
    let mut facts = SourceFacts::default();

    for source_file in &source_set.files {
        let text = fs::read_to_string(&source_file.path).map_err(|source| {
            SpecGenError::ReadSourceFile {
                path: source_file.path.clone(),
                source,
            }
        })?;
        facts
            .fc_apis
            .extend(extract_fc_apis(&text, &source_file.path));
        facts
            .classes
            .extend(extract_classes(&text, &source_file.path));
        facts.enums.extend(extract_enums(&text, &source_file.path));
        facts
            .static_variants
            .extend(extract_static_variants(&text, &source_file.path));
        facts
            .object_types
            .extend(extract_object_types(&text, &source_file.path));
        facts
            .reflects
            .extend(extract_reflects(&text, &source_file.path));
    }

    Ok(facts)
}
