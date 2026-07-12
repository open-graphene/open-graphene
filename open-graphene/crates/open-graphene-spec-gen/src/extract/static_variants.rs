use std::collections::BTreeMap;
use std::path::Path;

use super::facts::{RawStaticVariant, SourceLoc};
use super::lexer::{line_number, split_top_level_commas, strip_comments_preserving_newlines};

pub fn extract_static_variants(source_text: &str, file: &Path) -> Vec<RawStaticVariant> {
    let macros = extract_variadic_macros(source_text);
    let source = strip_comments_preserving_newlines(source_text);
    let mut out = Vec::new();
    out.extend(extract_using_static_variants(&source, file, &macros));
    out.extend(extract_typedef_static_variants(&source, file, &macros));
    out
}

fn extract_using_static_variants(
    source: &str,
    file: &Path,
    macros: &BTreeMap<String, Vec<String>>,
) -> Vec<RawStaticVariant> {
    let mut out = Vec::new();
    let mut offset = 0usize;

    while let Some(relative_start) = source[offset..].find("using ") {
        let using_start = offset + relative_start;
        let Some(eq_index) = source[using_start..].find('=').map(|idx| using_start + idx) else {
            offset = using_start + "using ".len();
            continue;
        };
        let name = source[using_start + "using ".len()..eq_index].trim();
        let Some(semicolon) = source[eq_index..].find(';').map(|idx| eq_index + idx) else {
            offset = eq_index + 1;
            continue;
        };
        let Some(static_start) = source[eq_index..semicolon]
            .find("static_variant")
            .or_else(|| source[eq_index..semicolon].find("fc::static_variant"))
            .map(|idx| eq_index + idx)
        else {
            offset = semicolon + 1;
            continue;
        };
        let Some((variants, end_index)) = parse_static_variant_args(source, static_start, macros)
        else {
            offset = static_start + 1;
            continue;
        };
        out.push(RawStaticVariant {
            name: name.to_string(),
            variants,
            source: SourceLoc {
                file: file.to_path_buf(),
                line: line_number(source, using_start),
            },
        });
        offset = end_index + 1;
    }

    out
}

fn extract_typedef_static_variants(
    source: &str,
    file: &Path,
    macros: &BTreeMap<String, Vec<String>>,
) -> Vec<RawStaticVariant> {
    let mut out = Vec::new();
    let mut offset = 0usize;

    while let Some(relative_start) = source[offset..].find("typedef ") {
        let typedef_start = offset + relative_start;
        let Some(static_start) = source[typedef_start..]
            .find("static_variant")
            .or_else(|| source[typedef_start..].find("fc::static_variant"))
            .map(|idx| typedef_start + idx)
        else {
            offset = typedef_start + "typedef ".len();
            continue;
        };
        let Some((variants, close_angle)) = parse_static_variant_args(source, static_start, macros)
        else {
            offset = static_start + 1;
            continue;
        };
        let Some(semicolon) = source[close_angle..].find(';').map(|idx| close_angle + idx) else {
            offset = close_angle + 1;
            continue;
        };
        let name = source[close_angle + 1..semicolon].trim();
        if !name.is_empty() {
            out.push(RawStaticVariant {
                name: name.to_string(),
                variants,
                source: SourceLoc {
                    file: file.to_path_buf(),
                    line: line_number(source, typedef_start),
                },
            });
        }
        offset = semicolon + 1;
    }

    out
}

fn parse_static_variant_args(
    source: &str,
    static_start: usize,
    macros: &BTreeMap<String, Vec<String>>,
) -> Option<(Vec<String>, usize)> {
    let open_angle = source[static_start..]
        .find('<')
        .map(|idx| static_start + idx)?;
    let close_angle = find_matching_angle(source, open_angle)?;
    let args = split_top_level_variant_args(&source[open_angle + 1..close_angle])
        .into_iter()
        .flat_map(|arg| macros.get(&arg).cloned().unwrap_or_else(|| vec![arg]))
        .collect();
    Some((args, close_angle))
}

fn extract_variadic_macros(source: &str) -> BTreeMap<String, Vec<String>> {
    let without_comments = strip_comments_preserving_newlines(source);
    let mut macros = BTreeMap::new();
    let lines = without_comments.lines().collect::<Vec<_>>();
    let mut index = 0usize;

    while index < lines.len() {
        let line = lines[index].trim_start();
        let Some(rest) = line.strip_prefix("#define ") else {
            index += 1;
            continue;
        };
        let Some((name, mut value)) = split_macro_name_and_value(rest) else {
            index += 1;
            continue;
        };

        while value.trim_end().ends_with('\\') && index + 1 < lines.len() {
            value = value
                .trim_end()
                .trim_end_matches('\\')
                .trim_end()
                .to_string();
            index += 1;
            value.push(' ');
            value.push_str(lines[index].trim());
        }
        value = value.trim_end().trim_end_matches('\\').trim().to_string();

        let variants = split_top_level_variant_args(&value);
        if !variants.is_empty() {
            macros.insert(name.to_string(), variants);
        }
        index += 1;
    }

    macros
}

/// Top-level comma split that also drops empty middle segments; variadic
/// macro expansions can leave dangling commas behind.
fn split_top_level_variant_args(source: &str) -> Vec<String> {
    split_top_level_commas(source)
        .into_iter()
        .filter(|segment| !segment.is_empty())
        .collect()
}

fn split_macro_name_and_value(rest: &str) -> Option<(&str, String)> {
    let mut parts = rest.splitn(2, char::is_whitespace);
    let name = parts.next()?.trim();
    let value = parts.next().unwrap_or_default().trim().to_string();
    if name.is_empty() || name.contains('(') {
        None
    } else {
        Some((name, value))
    }
}

fn find_matching_angle(source: &str, open_angle: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, ch) in source
        .char_indices()
        .skip_while(|(idx, _)| *idx < open_angle)
    {
        match ch {
            '<' => depth += 1,
            '>' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn extracts_using_fc_static_variant() {
        let source = r#"
            using operation = fc::static_variant<
               transfer_operation,
               limit_order_create_operation
            >;
        "#;

        let variants = extract_static_variants(source, &PathBuf::from("operations.hpp"));
        assert_eq!(variants.len(), 1);
        assert_eq!(variants[0].name, "operation");
        assert_eq!(
            variants[0].variants,
            vec!["transfer_operation", "limit_order_create_operation"]
        );
    }

    #[test]
    fn does_not_cross_using_statement_boundaries() {
        let source = r#"
            using extendable_operation_result = extension<extendable_operation_result_dtl>;
            using operation_result = fc::static_variant <
               void_result,
               object_id_type
            >;
        "#;

        let variants = extract_static_variants(source, &PathBuf::from("base.hpp"));
        assert_eq!(variants.len(), 1);
        assert_eq!(variants[0].name, "operation_result");
        assert_eq!(variants[0].variants, vec!["void_result", "object_id_type"]);
    }

    #[test]
    fn extracts_typedef_static_variant() {
        let source = r#"
            typedef static_variant<
               refund_worker_type,
               vesting_balance_worker_type
            > worker_initializer;
        "#;

        let variants = extract_static_variants(source, &PathBuf::from("worker.hpp"));
        assert_eq!(variants.len(), 1);
        assert_eq!(variants[0].name, "worker_initializer");
        assert_eq!(
            variants[0].variants,
            vec!["refund_worker_type", "vesting_balance_worker_type"]
        );
    }

    #[test]
    fn expands_variadic_macro_static_variant_arms() {
        let source = r#"
            #define GRAPHENE_OP_RESTRICTION_ARGUMENTS_VARIADIC \
               void_t, \
               bool, \
               flat_set<account_id_type>, \
               variant_assert_argument_type

            using argument_type = fc::static_variant<GRAPHENE_OP_RESTRICTION_ARGUMENTS_VARIADIC>;
        "#;

        let variants = extract_static_variants(source, &PathBuf::from("restriction.hpp"));
        assert_eq!(variants.len(), 1);
        assert_eq!(variants[0].name, "argument_type");
        assert_eq!(
            variants[0].variants,
            vec![
                "void_t",
                "bool",
                "flat_set<account_id_type>",
                "variant_assert_argument_type"
            ]
        );
    }
}
