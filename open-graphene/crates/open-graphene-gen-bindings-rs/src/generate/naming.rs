use super::*;

pub(crate) fn object_id_type_name(object_type: &str) -> String {
    format!("{}Id", rust_type_name(object_type))
}

pub(crate) fn openapi_schema_name(protocol: &Protocol, rust_name: &str) -> String {
    format!(
        "Graphene{}{}",
        rust_type_name(&protocol.chain.id),
        rust_name
    )
}

pub(crate) fn rust_type_name(value: &str) -> String {
    let words = words(value);
    let mut out = String::new();
    for word in words {
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            out.extend(first.to_uppercase());
            out.push_str(chars.as_str());
        }
    }
    if out.is_empty() {
        "GeneratedType".to_string()
    } else if out.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        format!("N{out}")
    } else if is_rust_keyword(&out) {
        format!("{out}Type")
    } else {
        out
    }
}

pub(crate) fn rust_variant_name(value: &str) -> String {
    let name = rust_type_name(value);
    if is_rust_keyword(&name) {
        format!("{name}Variant")
    } else {
        name
    }
}

pub(crate) fn rust_const_name(value: &str) -> String {
    let words = words(value);
    let mut out = words
        .into_iter()
        .map(|word| word.to_ascii_uppercase())
        .collect::<Vec<_>>()
        .join("_");
    if out.is_empty() {
        out.push_str("GENERATED_CONST");
    }
    if out.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        out.insert(0, 'N');
    }
    out
}

pub(crate) fn rust_field_name(value: &str) -> String {
    let words = words(value);
    let mut out = words.join("_");
    if out.is_empty() {
        out.push_str("generated_field");
    }
    if out.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        out.insert(0, '_');
    }
    if is_rust_keyword(&out) {
        format!("r#{out}")
    } else {
        out
    }
}

pub(crate) fn words(value: &str) -> Vec<String> {
    value
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(|part| part.to_ascii_lowercase())
        .collect()
}

pub(crate) fn serde_rename_attr(indent: &str, original: &str, generated: &str) -> String {
    let normalized_generated = generated.strip_prefix("r#").unwrap_or(generated);
    if original == normalized_generated {
        String::new()
    } else {
        format!(
            "{indent}#[serde(rename = {})]\n",
            rust_string_literal(original)
        )
    }
}

pub(crate) fn is_rust_keyword(value: &str) -> bool {
    matches!(
        value,
        "as" | "break"
            | "const"
            | "continue"
            | "crate"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "Self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
            | "async"
            | "await"
            | "dyn"
            | "abstract"
            | "become"
            | "box"
            | "do"
            | "final"
            | "macro"
            | "override"
            | "priv"
            | "try"
            | "typeof"
            | "unsized"
            | "virtual"
            | "yield"
    )
}

pub(crate) fn rust_string_literal(value: &str) -> String {
    // std's `Debug` for `str` produces a valid Rust string literal; JSON escaping
    // (`\b`, `\f`, bare `\uXXXX`) is not valid Rust.
    format!("{value:?}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_names_handle_keywords_and_protocol_case() {
        assert_eq!(
            rust_type_name("account_create_operation"),
            "AccountCreateOperation"
        );
        assert_eq!(rust_field_name("type"), "r#type");
        assert_eq!(
            rust_const_name("transfer_operation_id"),
            "TRANSFER_OPERATION_ID"
        );
        assert_eq!(object_id_type_name("account"), "AccountId");
        // A spec type named `self` must not emit `struct Self`.
        assert_eq!(rust_type_name("self"), "SelfType");
    }

    #[test]
    fn rust_string_literal_emits_valid_rust_escapes() {
        assert_eq!(rust_string_literal("plain"), "\"plain\"");
        assert_eq!(
            rust_string_literal("with \"quotes\" and \\ backslash"),
            r#""with \"quotes\" and \\ backslash""#
        );
        assert_eq!(
            rust_string_literal("tab\tnewline\n"),
            "\"tab\\tnewline\\n\""
        );
        // JSON would emit `\b`, `\f`, or bare `\uXXXX` here, none of which are
        // valid Rust escapes; std Debug uses `\u{..}` instead.
        assert_eq!(rust_string_literal("\u{8}\u{c}"), "\"\\u{8}\\u{c}\"");
    }
}
