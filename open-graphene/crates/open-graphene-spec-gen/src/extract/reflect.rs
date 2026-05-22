use std::path::Path;

use super::facts::{RawReflect, SourceLoc};

pub fn extract_reflects(source_text: &str, file: &Path) -> Vec<RawReflect> {
    let source = strip_comments_preserving_newlines(source_text);
    let mut reflects = Vec::new();
    let mut offset = 0usize;

    while let Some((macro_start, macro_name)) = find_next_reflect_macro(&source, offset) {
        let Some(open_paren) = source[macro_start..].find('(').map(|idx| macro_start + idx) else {
            offset = macro_start + macro_name.len();
            continue;
        };
        let Some(close_paren) = find_matching_paren(&source, open_paren) else {
            offset = open_paren + 1;
            continue;
        };
        let args = split_top_level_commas(&source[open_paren + 1..close_paren]);
        let field_arg_index = if macro_name == "FC_REFLECT_DERIVED" {
            2
        } else {
            1
        };
        if let Some(type_name) = args.first() {
            let fields = args
                .get(field_arg_index)
                .map(|fields_arg| parse_reflect_fields(fields_arg))
                .unwrap_or_default();
            reflects.push(RawReflect {
                type_name: type_name.trim().to_string(),
                fields,
                source: SourceLoc {
                    file: file.to_path_buf(),
                    line: line_number(&source, macro_start),
                },
                derived: macro_name == "FC_REFLECT_DERIVED",
            });
        }
        offset = close_paren + 1;
    }

    reflects
}

fn find_next_reflect_macro(source: &str, offset: usize) -> Option<(usize, &'static str)> {
    let reflect = source[offset..]
        .find("FC_REFLECT(")
        .map(|relative| (offset + relative, "FC_REFLECT"));
    let derived = source[offset..]
        .find("FC_REFLECT_DERIVED(")
        .map(|relative| (offset + relative, "FC_REFLECT_DERIVED"));

    match (reflect, derived) {
        (Some(reflect), Some(derived)) => {
            Some(std::cmp::min_by_key(reflect, derived, |(idx, _)| *idx))
        }
        (Some(reflect), None) => Some(reflect),
        (None, Some(derived)) => Some(derived),
        (None, None) => None,
    }
}

fn parse_reflect_fields(source: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut offset = 0usize;

    while let Some(relative_open) = source[offset..].find('(') {
        let open = offset + relative_open;
        let Some(close) = find_matching_paren(source, open) else {
            break;
        };
        let field = source[open + 1..close].trim();
        if !field.is_empty() {
            fields.push(field.to_string());
        }
        offset = close + 1;
    }

    fields
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
    fn extracts_fc_reflect_fields() {
        let source = r#"
            FC_REFLECT( graphene::protocol::transfer_operation,
                        (fee)(from)(to)(amount)(memo)(extensions) )
        "#;

        let reflects = extract_reflects(source, &PathBuf::from("transfer.hpp"));

        assert_eq!(reflects.len(), 1);
        assert_eq!(
            reflects[0].type_name,
            "graphene::protocol::transfer_operation"
        );
        assert_eq!(
            reflects[0].fields,
            vec!["fee", "from", "to", "amount", "memo", "extensions"]
        );
        assert!(!reflects[0].derived);
    }

    #[test]
    fn extracts_fc_reflect_derived_fields() {
        let source = r#"
            FC_REFLECT_DERIVED( graphene::chain::operation_history_object,
                                (graphene::db::abstract_object<operation_history_object>),
                                (op)(result)(block_num)(trx_in_block) )
        "#;

        let reflects = extract_reflects(source, &PathBuf::from("operation_history_object.hpp"));

        assert_eq!(reflects.len(), 1);
        assert_eq!(
            reflects[0].type_name,
            "graphene::chain::operation_history_object"
        );
        assert_eq!(
            reflects[0].fields,
            vec!["op", "result", "block_num", "trx_in_block"]
        );
        assert!(reflects[0].derived);
    }

    #[test]
    fn extracts_empty_reflect_field_list() {
        let source = "FC_REFLECT(graphene::protocol::void_t,)";

        let reflects = extract_reflects(source, &PathBuf::from("types.hpp"));

        assert_eq!(reflects.len(), 1);
        assert!(reflects[0].fields.is_empty());
    }
}
