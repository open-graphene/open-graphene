use std::path::Path;

use super::facts::{FcApi, SourceLoc};

pub fn extract_fc_apis(source_text: &str, file: &Path) -> Vec<FcApi> {
    find_invocations(source_text, "FC_API")
        .into_iter()
        .filter_map(|invocation| {
            if invocation.args.len() != 2 {
                return None;
            }

            let method_names = parse_paren_list(&invocation.args[1]);
            if method_names.is_empty() {
                return None;
            }

            Some(FcApi {
                api_class: invocation.args[0].trim().to_string(),
                method_names,
                source: SourceLoc {
                    file: file.to_path_buf(),
                    line: invocation.line,
                },
            })
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MacroInvocation {
    args: Vec<String>,
    line: usize,
}

fn find_invocations(source: &str, macro_name: &str) -> Vec<MacroInvocation> {
    let mut out = Vec::new();
    let mut offset = 0;

    while let Some(relative_start) = source[offset..].find(macro_name) {
        let macro_start = offset + relative_start;
        if !is_macro_boundary(source, macro_start, macro_name.len()) {
            offset = macro_start + macro_name.len();
            continue;
        }

        let Some(open_paren) = source[macro_start + macro_name.len()..].find('(') else {
            break;
        };
        let open_paren = macro_start + macro_name.len() + open_paren;

        let Some(close_paren) = find_matching_paren(source, open_paren) else {
            offset = open_paren + 1;
            continue;
        };

        let args_source = &source[open_paren + 1..close_paren];
        out.push(MacroInvocation {
            args: split_top_level_commas(args_source),
            line: line_number(source, macro_start),
        });
        offset = close_paren + 1;
    }

    out
}

fn is_macro_boundary(source: &str, start: usize, len: usize) -> bool {
    let before_ok = source[..start]
        .chars()
        .next_back()
        .is_none_or(|ch| !is_ident_char(ch));
    let after_ok = source[start + len..]
        .chars()
        .next()
        .is_none_or(|ch| !is_ident_char(ch));
    before_ok && after_ok
}

fn is_ident_char(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}

fn find_matching_paren(source: &str, open_paren: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut depth = 0usize;
    for (index, byte) in bytes.iter().enumerate().skip(open_paren) {
        match byte {
            b'(' => depth += 1,
            b')' => {
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

fn parse_paren_list(source: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = source.as_bytes();
    let mut index = 0usize;

    while index < bytes.len() {
        if bytes[index] != b'(' {
            index += 1;
            continue;
        }

        let Some(close) = find_matching_paren(source, index) else {
            break;
        };
        let value = source[index + 1..close].trim();
        if !value.is_empty() {
            out.push(value.to_string());
        }
        index = close + 1;
    }

    out
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
    fn extracts_fc_api_methods() {
        let source = r#"
            namespace graphene { namespace app {}
            FC_API(graphene::app::database_api,
               (get_objects)
               (get_block)
            )
        "#;

        let apis = extract_fc_apis(source, &PathBuf::from("database_api.hpp"));
        assert_eq!(apis.len(), 1);
        assert_eq!(apis[0].api_class, "graphene::app::database_api");
        assert_eq!(apis[0].method_names, vec!["get_objects", "get_block"]);
        assert_eq!(apis[0].source.line, 3);
    }

    #[test]
    fn ignores_identifier_substrings() {
        let source = "NOT_FC_API(foo, (bar))\nFC_API(app::history_api, (get_account_history))";
        let apis = extract_fc_apis(source, &PathBuf::from("api.hpp"));
        assert_eq!(apis.len(), 1);
        assert_eq!(apis[0].api_class, "app::history_api");
    }
}
