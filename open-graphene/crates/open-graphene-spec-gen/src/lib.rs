//! Open Graphene protocol specification generator.
//!
//! This crate is intentionally minimal for now. It is the future home for
//! source-code extractors that emit `open_graphene_json_schema::Protocol`
//! documents.

/// Library marker used by the empty crate smoke test.
pub fn crate_ready() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_is_ready() {
        assert!(crate_ready());
    }
}
