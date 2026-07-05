//! Detect chain-emitted (virtual) operations.
//!
//! Graphene marks virtual operations with a trailing `// VIRTUAL` comment on the
//! arm inside the `operation` static-variant typedef (`operations.hpp`). The
//! static-variant parser strips comments before parsing, so the marker has to be
//! collected in a separate pass over the raw source text.

/// Names of types whose declaration line carries a `// VIRTUAL` marker.
///
/// A line qualifies when, after removing inline `/* .. */` comments, the code
/// before the `//` comment is a bare identifier (optionally followed by a
/// comma) and the comment contains the word `VIRTUAL`.
pub fn extract_virtual_operation_markers(source_text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in source_text.lines() {
        let Some((code, comment)) = line.split_once("//") else {
            continue;
        };
        let has_marker = comment
            .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
            .any(|word| word == "VIRTUAL");
        if !has_marker {
            continue;
        }
        let code = strip_inline_block_comments(code);
        let name = code.trim().trim_end_matches(',').trim();
        if !name.is_empty()
            && name
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        {
            out.push(name.to_string());
        }
    }
    out
}

fn strip_inline_block_comments(code: &str) -> String {
    let mut output = String::with_capacity(code.len());
    let mut rest = code;
    while let Some(open) = rest.find("/*") {
        output.push_str(&rest[..open]);
        match rest[open + 2..].find("*/") {
            Some(close) => rest = &rest[open + 2 + close + 2..],
            None => return output,
        }
    }
    output.push_str(rest);
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_marked_operation_arms() {
        let source = r#"
            /*  3 */ call_order_update_operation,
            /*  4 */ fill_order_operation,           // VIRTUAL
            /*  5 */ account_create_operation,
            /* 42 */ asset_settle_cancel_operation,  // VIRTUAL
        "#;

        assert_eq!(
            extract_virtual_operation_markers(source),
            vec!["fill_order_operation", "asset_settle_cancel_operation"]
        );
    }

    #[test]
    fn ignores_comments_without_the_marker_word() {
        let source = r#"
            transfer_operation, // virtually free
            htlc_refund_operation, // VIRTUALIZED
        "#;

        assert!(extract_virtual_operation_markers(source).is_empty());
    }

    #[test]
    fn ignores_marker_lines_that_are_not_bare_identifiers() {
        let source = "typedef static_variant< a, b > ops; // VIRTUAL";

        assert!(extract_virtual_operation_markers(source).is_empty());
    }
}
