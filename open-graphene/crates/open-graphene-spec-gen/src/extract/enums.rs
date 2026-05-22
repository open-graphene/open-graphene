use std::path::Path;

use super::facts::{RawEnum, RawEnumValue, SourceLoc};

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
}
