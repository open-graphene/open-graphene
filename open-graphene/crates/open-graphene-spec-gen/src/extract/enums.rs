use std::collections::BTreeMap;
use std::path::Path;

use super::facts::{RawEnum, RawEnumDefinition, RawEnumMember, RawEnumValue, SourceLoc};

pub fn extract_enums(source_text: &str, file: &Path) -> Vec<RawEnum> {
    let source = strip_comments_preserving_newlines(source_text);
    let mut enums = Vec::new();
    let mut offset = 0usize;

    while let Some(relative_start) = source[offset..].find("FC_REFLECT_ENUM(") {
        let macro_start = offset + relative_start;
        let Some(open_paren) = source[macro_start..].find('(').map(|idx| macro_start + idx) else {
            offset = macro_start + "FC_REFLECT_ENUM".len();
            continue;
        };
        let Some(close_paren) = find_matching_paren(&source, open_paren) else {
            offset = open_paren + 1;
            continue;
        };
        let args = split_top_level_commas(&source[open_paren + 1..close_paren]);
        if let (Some(name), Some(values_arg)) = (args.first(), args.get(1)) {
            let name = name.trim();
            if is_preprocessor_generated_name(name) {
                offset = close_paren + 1;
                continue;
            }
            enums.push(RawEnum {
                name: name.to_string(),
                values: parse_enum_values(values_arg),
                source: SourceLoc {
                    file: file.to_path_buf(),
                    line: line_number(&source, macro_start),
                },
            });
        }
        offset = close_paren + 1;
    }

    enums
}

fn is_preprocessor_generated_name(name: &str) -> bool {
    name.contains("BOOST_PP")
        || name.contains("object_type_prefix")
        || name.contains("id_namespace")
}

/// Extracts C++ `enum` bodies with their real member values. FC_REFLECT_ENUM
/// never carries values, so these definitions are the only source of truth
/// for numeric enum members (bitflags especially).
pub fn extract_enum_definitions(source_text: &str, file: &Path) -> Vec<RawEnumDefinition> {
    let source = strip_comments_preserving_newlines(source_text);
    let mut out = Vec::new();
    let mut offset = 0usize;

    while let Some(relative_start) = source[offset..].find("enum ") {
        let enum_start = offset + relative_start;
        if !is_word_boundary_before(&source, enum_start) {
            offset = enum_start + "enum ".len();
            continue;
        }
        let mut cursor = enum_start + "enum ".len();
        // Optional `class`/`struct` scoping keyword.
        for keyword in ["class ", "struct "] {
            if source[cursor..].starts_with(keyword) {
                cursor += keyword.len();
            }
        }
        let rest = &source[cursor..];
        let name_len = rest
            .char_indices()
            .find(|(_, ch)| !ch.is_alphanumeric() && *ch != '_')
            .map(|(idx, _)| idx)
            .unwrap_or(rest.len());
        let name = rest[..name_len].trim();
        if name.is_empty() {
            offset = cursor;
            continue;
        }
        cursor += name_len;
        // Optional underlying type (`: uint8_t`), then the body must open
        // before any `;` (otherwise it is a forward declaration).
        let Some(open_brace) = source[cursor..].find('{').map(|idx| cursor + idx) else {
            offset = cursor;
            continue;
        };
        if source[cursor..open_brace].contains(';') {
            offset = cursor;
            continue;
        }
        let Some(close_brace) = find_matching_brace(&source, open_brace) else {
            offset = open_brace + 1;
            continue;
        };

        let members = parse_enum_definition_members(&source[open_brace + 1..close_brace]);
        if !members.is_empty() {
            out.push(RawEnumDefinition {
                name: name.to_string(),
                members,
                source: SourceLoc {
                    file: file.to_path_buf(),
                    line: line_number(&source, enum_start),
                },
            });
        }
        offset = close_brace + 1;
    }

    out
}

fn is_word_boundary_before(source: &str, offset: usize) -> bool {
    source[..offset]
        .chars()
        .next_back()
        .is_none_or(|ch| !ch.is_alphanumeric() && ch != '_')
}

fn find_matching_brace(source: &str, open_brace: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, ch) in source
        .char_indices()
        .skip_while(|(idx, _)| *idx < open_brace)
    {
        match ch {
            '{' => depth += 1,
            '}' => {
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

/// Splits an enum body on commas, tracking parentheses only: `<`/`>` are
/// shift operators inside initializers, never brackets.
fn split_enum_body_commas(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut start = 0usize;
    let mut paren_depth = 0usize;
    for (index, ch) in body.char_indices() {
        match ch {
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            ',' if paren_depth == 0 => {
                out.push(body[start..index].trim().to_string());
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }
    let trailing = body[start..].trim();
    if !trailing.is_empty() {
        out.push(trailing.to_string());
    }
    out
}

fn parse_enum_definition_members(body: &str) -> Vec<RawEnumMember> {
    let mut members = Vec::new();
    let mut resolved: BTreeMap<String, i64> = BTreeMap::new();
    let mut next_implicit: Option<i64> = Some(0);

    for entry in split_enum_body_commas(body) {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }
        let (name, value) = match entry.split_once('=') {
            Some((name, expr)) => (name.trim(), evaluate_enum_expr(expr.trim(), &resolved)),
            None => (entry, next_implicit),
        };
        if name.is_empty() || name.contains(|ch: char| !ch.is_alphanumeric() && ch != '_') {
            // Preprocessor-generated or otherwise unparseable member: keep the
            // definition but poison implicit numbering from here on.
            next_implicit = None;
            continue;
        }
        if let Some(value) = value {
            resolved.insert(name.to_string(), value);
        }
        next_implicit = value.and_then(|value| value.checked_add(1));
        members.push(RawEnumMember {
            name: name.to_string(),
            value,
        });
    }

    members
}

/// Evaluates the small expression grammar real Graphene enums use:
/// integer literals (decimal/hex), references to earlier members, and the
/// binary operators `|`, `&`, `^`, `<<`, `+`, `-`. Anything else yields `None`.
fn evaluate_enum_expr(expr: &str, resolved: &BTreeMap<String, i64>) -> Option<i64> {
    let expr = strip_outer_parens(expr.trim());
    // Lowest-precedence operator first, so the split mirrors C precedence.
    for operator in ["|", "^", "&", "<<", "+", "-"] {
        if let Some((left, right)) = split_binary_expr(expr, operator) {
            let left = evaluate_enum_expr(&left, resolved)?;
            let right = evaluate_enum_expr(&right, resolved)?;
            return match operator {
                "|" => Some(left | right),
                "^" => Some(left ^ right),
                "&" => Some(left & right),
                "<<" => left.checked_shl(u32::try_from(right).ok()?),
                "+" => left.checked_add(right),
                "-" => left.checked_sub(right),
                _ => unreachable!(),
            };
        }
    }
    evaluate_enum_atom(expr, resolved)
}

fn strip_outer_parens(expr: &str) -> &str {
    let mut expr = expr;
    while expr.starts_with('(') && expr.ends_with(')') {
        let mut depth = 0usize;
        let mut wraps_whole_expr = true;
        for (index, ch) in expr.char_indices() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 && index != expr.len() - 1 {
                        wraps_whole_expr = false;
                        break;
                    }
                }
                _ => {}
            }
        }
        if !wraps_whole_expr {
            break;
        }
        expr = expr[1..expr.len() - 1].trim();
    }
    expr
}

/// Splits at the last top-level occurrence of `operator` so same-precedence
/// chains associate left, matching C.
fn split_binary_expr(expr: &str, operator: &str) -> Option<(String, String)> {
    let mut depth = 0usize;
    let mut split_at = None;
    let bytes = expr.as_bytes();
    for index in 0..bytes.len() {
        match bytes[index] {
            b'(' => depth += 1,
            b')' => depth = depth.saturating_sub(1),
            _ if depth == 0 && expr[index..].starts_with(operator) => {
                // Index 0 would be a unary operator, not a binary split point.
                if index > 0 {
                    split_at = Some(index);
                }
            }
            _ => {}
        }
    }
    let index = split_at?;
    Some((
        expr[..index].trim().to_string(),
        expr[index + operator.len()..].trim().to_string(),
    ))
}

fn evaluate_enum_atom(atom: &str, resolved: &BTreeMap<String, i64>) -> Option<i64> {
    let atom = atom.trim();
    if let Some(hex) = atom.strip_prefix("0x").or_else(|| atom.strip_prefix("0X")) {
        return i64::from_str_radix(hex, 16).ok();
    }
    if let Ok(value) = atom.parse::<i64>() {
        return Some(value);
    }
    resolved.get(atom.trim_start_matches("::")).copied()
}

fn parse_enum_values(source: &str) -> Vec<RawEnumValue> {
    let mut values = Vec::new();
    let mut offset = 0usize;

    while let Some(relative_open) = source[offset..].find('(') {
        let open = offset + relative_open;
        let Some(close) = find_matching_paren(source, open) else {
            break;
        };
        let raw = source[open + 1..close].trim();
        if !raw.is_empty() {
            values.push(parse_enum_value(raw, values.len() as i64));
        }
        offset = close + 1;
    }

    values
}

fn parse_enum_value(raw: &str, fallback_value: i64) -> RawEnumValue {
    if let Some((name, value)) = raw.split_once('=') {
        RawEnumValue {
            name: name.trim().to_string(),
            value: value.trim().parse::<i64>().ok().unwrap_or(fallback_value),
        }
    } else {
        RawEnumValue {
            name: raw.trim().to_string(),
            value: fallback_value,
        }
    }
}

fn split_top_level_commas(source: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut start = 0usize;
    let mut paren_depth = 0usize;
    let mut angle_depth = 0usize;

    for (index, ch) in source.char_indices() {
        match ch {
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '<' => angle_depth += 1,
            '>' => angle_depth = angle_depth.saturating_sub(1),
            ',' if paren_depth == 0 && angle_depth == 0 => {
                out.push(source[start..index].trim().to_string());
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }

    let trailing = source[start..].trim();
    if !trailing.is_empty() {
        out.push(trailing.to_string());
    }
    out
}

fn find_matching_paren(source: &str, open_paren: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, ch) in source
        .char_indices()
        .skip_while(|(idx, _)| *idx < open_paren)
    {
        match ch {
            '(' => depth += 1,
            ')' => {
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

fn strip_comments_preserving_newlines(source: &str) -> String {
    let mut output = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '/' && chars.peek() == Some(&'/') {
            chars.next();
            for comment_ch in chars.by_ref() {
                if comment_ch == '\n' {
                    output.push('\n');
                    break;
                }
            }
        } else if ch == '/' && chars.peek() == Some(&'*') {
            chars.next();
            let mut previous = '\0';
            for comment_ch in chars.by_ref() {
                if comment_ch == '\n' {
                    output.push('\n');
                }
                if previous == '*' && comment_ch == '/' {
                    break;
                }
                previous = comment_ch;
            }
        } else {
            output.push(ch);
        }
    }

    output
}

fn line_number(source: &str, offset: usize) -> usize {
    source[..offset]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn extracts_fc_reflect_enum_values() {
        let source = r#"
            FC_REFLECT_ENUM(graphene::protocol::restriction::function_type,
                            (func_eq)(func_ne)(func_lt))
        "#;

        let enums = extract_enums(source, &PathBuf::from("restriction.hpp"));

        assert_eq!(enums.len(), 1);
        assert_eq!(
            enums[0].name,
            "graphene::protocol::restriction::function_type"
        );
        assert_eq!(
            enums[0].values,
            vec![
                RawEnumValue {
                    name: "func_eq".to_string(),
                    value: 0
                },
                RawEnumValue {
                    name: "func_ne".to_string(),
                    value: 1
                },
                RawEnumValue {
                    name: "func_lt".to_string(),
                    value: 2
                },
            ]
        );
    }

    #[test]
    fn ignores_fc_reflect_enum_inside_preprocessor_templates() {
        let source = r#"
            #define GRAPHENE_DEFINE_IDS(id_namespace, object_space, object_type_prefix, names_seq) \
               FC_REFLECT_ENUM(graphene::id_namespace::BOOST_PP_CAT(object_type_prefix, object_type), \
                               BOOST_PP_SEQ_TRANSFORM(NAME_TO_OBJECT_TYPE, object_type_prefix, names_seq))
        "#;

        let enums = extract_enums(source, &PathBuf::from("types.hpp"));

        assert!(enums.is_empty());
    }

    #[test]
    fn extracts_explicit_enum_value_when_present() {
        let source = "FC_REFLECT_ENUM(graphene::protocol::sample, (first)(custom = 7)(next))";

        let enums = extract_enums(source, &PathBuf::from("sample.hpp"));

        assert_eq!(enums[0].values[0].value, 0);
        assert_eq!(enums[0].values[1].name, "custom");
        assert_eq!(enums[0].values[1].value, 7);
        assert_eq!(enums[0].values[2].value, 2);
    }

    #[test]
    fn extracts_enum_definition_with_hex_bitflags() {
        let source = r#"
            enum asset_issuer_permission_flags {
               charge_market_fee    = 0x01, ///< market trades may be charged
               white_list           = 0x02,
               override_authority   = 0x04,
               transfer_restricted  = 0x08,
               disable_collateral_bidding = 0x8000
            };
        "#;

        let definitions = extract_enum_definitions(source, &PathBuf::from("types.hpp"));

        assert_eq!(definitions.len(), 1);
        assert_eq!(definitions[0].name, "asset_issuer_permission_flags");
        let values: Vec<(&str, Option<i64>)> = definitions[0]
            .members
            .iter()
            .map(|member| (member.name.as_str(), member.value))
            .collect();
        assert_eq!(
            values,
            vec![
                ("charge_market_fee", Some(0x01)),
                ("white_list", Some(0x02)),
                ("override_authority", Some(0x04)),
                ("transfer_restricted", Some(0x08)),
                ("disable_collateral_bidding", Some(0x8000)),
            ]
        );
    }

    #[test]
    fn extracts_enum_definition_with_implicit_and_expression_values() {
        let source = r#"
            enum class account_listing : uint8_t {
               no_listing = 0x0,
               white_listed = 0x1,
               black_listed = 0x2,
               white_and_black_listed = white_listed | black_listed,
               shifted = 1 << 4,
               implicit_after_shifted
            };
        "#;

        let definitions = extract_enum_definitions(source, &PathBuf::from("account.hpp"));

        assert_eq!(definitions.len(), 1);
        assert_eq!(definitions[0].name, "account_listing");
        let members = &definitions[0].members;
        assert_eq!(members[3].value, Some(0x3));
        assert_eq!(members[4].value, Some(16));
        assert_eq!(members[5].value, Some(17));
    }

    #[test]
    fn implicit_enum_values_continue_from_previous_member() {
        let source = "enum sample { first = 5, second, third };";

        let definitions = extract_enum_definitions(source, &PathBuf::from("sample.hpp"));

        let values: Vec<Option<i64>> = definitions[0]
            .members
            .iter()
            .map(|member| member.value)
            .collect();
        assert_eq!(values, vec![Some(5), Some(6), Some(7)]);
    }

    #[test]
    fn unevaluatable_initializer_yields_none_and_poisons_followers() {
        let source = "enum sample { first = SOME_MACRO, second, third = 9 };";

        let definitions = extract_enum_definitions(source, &PathBuf::from("sample.hpp"));

        let values: Vec<Option<i64>> = definitions[0]
            .members
            .iter()
            .map(|member| member.value)
            .collect();
        assert_eq!(values, vec![None, None, Some(9)]);
    }

    #[test]
    fn ignores_enum_forward_declarations() {
        let source = "enum sample_status : int; struct holder { };";

        let definitions = extract_enum_definitions(source, &PathBuf::from("fwd.hpp"));

        assert!(definitions.is_empty());
    }
}
