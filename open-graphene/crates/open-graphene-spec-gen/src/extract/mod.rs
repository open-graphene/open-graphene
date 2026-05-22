pub mod classes;
pub mod facts;
pub mod macros;

use std::fs;

use crate::error::{Result, SpecGenError};
use crate::source::SourceSet;

pub use classes::extract_classes;
pub use facts::{ExtractDiagnostic, FcApi, RawClass, RawMethod, RawParam, SourceFacts, SourceLoc};
pub use macros::extract_fc_apis;

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
    }

    Ok(facts)
}
