use std::path::Path;

use super::facts::{RawClass, RawMethod, RawParam, SourceLoc};

pub fn extract_classes(source_text: &str, file: &Path) -> Vec<RawClass> {
    let source = strip_comments_preserving_newlines(source_text);
    let mut classes = Vec::new();
    let mut offset = 0usize;

    while let Some(relative_start) = source[offset..].find("class ") {
        let class_start = offset + relative_start;
        if !is_keyword_boundary(&source, class_start, "class".len()) {
            offset = class_start + "class".len();
            continue;
        }

        let Some((name, after_name)) = parse_class_name(&source, class_start + "class".len())
        else {
            offset = class_start + "class".len();
            continue;
        };
        let next_open_brace = source[after_name..].find('{').map(|idx| after_name + idx);
        let next_semicolon = source[after_name..].find(';').map(|idx| after_name + idx);
        if next_semicolon.is_some_and(|semicolon| {
            next_open_brace.is_none_or(|open_brace| semicolon < open_brace)
        }) {
            offset = next_semicolon.expect("checked above") + 1;
            continue;
        }
        let Some(open_brace) = next_open_brace else {
            offset = after_name;
            continue;
        };
        let Some(close_brace) = find_matching_brace(&source, open_brace) else {
            offset = open_brace + 1;
            continue;
        };

        let class_line = line_number(&source, class_start);
        let body = &source[open_brace + 1..close_brace];
        classes.push(RawClass {
            name,
            qualified_name: None,
            methods: extract_methods_from_class_body(body, file, class_line),
            source: SourceLoc {
                file: file.to_path_buf(),
                line: class_line,
            },
        });
        offset = close_brace + 1;
    }

    classes
}

fn extract_methods_from_class_body(body: &str, file: &Path, class_line: usize) -> Vec<RawMethod> {
    let mut methods = Vec::new();
    let mut statement_start = 0usize;
    let mut paren_depth = 0usize;
    let mut angle_depth = 0usize;
    let mut brace_depth = 0usize;

    for (index, ch) in body.char_indices() {
        match ch {
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '<' => angle_depth += 1,
            '>' => angle_depth = angle_depth.saturating_sub(1),
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            ';' if paren_depth == 0 && angle_depth == 0 && brace_depth == 0 => {
                let statement = &body[statement_start..index];
                let statement_line = class_line + line_number(body, statement_start) - 1;
                if let Some(method) = parse_method_statement(statement, file, statement_line) {
                    methods.push(method);
                }
                statement_start = index + ch.len_utf8();
            }
            _ => {}
        }
    }

    methods
}

fn parse_method_statement(statement: &str, file: &Path, line: usize) -> Option<RawMethod> {
    let statement = collapse_whitespace(statement);
    let statement = strip_access_labels(&statement).trim().to_string();
    if statement.is_empty()
        || statement.starts_with("typedef ")
        || statement.starts_with("using ")
        || statement.starts_with("friend ")
        || statement.starts_with("enum ")
        || statement.starts_with("struct ")
    {
        return None;
    }

    let open_paren = statement.find('(')?;
    let close_paren = find_matching_paren(&statement, open_paren)?;
    let before_params = statement[..open_paren].trim();
    let after_params = statement[close_paren + 1..].trim();
    let (return_type, name) = split_return_type_and_name(before_params)?;

    if name.starts_with('~') || return_type.is_empty() || name == return_type {
        return None;
    }

    let params = parse_params(&statement[open_paren + 1..close_paren]);

    Some(RawMethod {
        name: name.to_string(),
        return_type: return_type.to_string(),
        params,
        is_const: after_params
            .split_whitespace()
            .any(|token| token == "const"),
        source: SourceLoc {
            file: file.to_path_buf(),
            line,
        },
    })
}

fn strip_access_labels(mut statement: &str) -> &str {
    loop {
        let trimmed = statement.trim_start();
        if let Some(rest) = trimmed.strip_prefix("public:") {
            statement = rest;
        } else if let Some(rest) = trimmed.strip_prefix("private:") {
            statement = rest;
        } else if let Some(rest) = trimmed.strip_prefix("protected:") {
            statement = rest;
        } else {
            return trimmed;
        }
    }
}

fn split_return_type_and_name(before_params: &str) -> Option<(&str, &str)> {
    let end = before_params.trim_end().len();
    let trimmed = &before_params[..end];
    let name_start = trimmed
        .char_indices()
        .rev()
        .find(|(_, ch)| !(ch.is_ascii_alphanumeric() || *ch == '_'))
        .map_or(0, |(index, ch)| index + ch.len_utf8());
    let name = trimmed[name_start..].trim();
    let return_type = trimmed[..name_start].trim();
    if name.is_empty() {
        None
    } else {
        Some((return_type, name))
    }
}

fn parse_params(params: &str) -> Vec<RawParam> {
    split_top_level(params, ',')
        .into_iter()
        .filter_map(|param| parse_param(&param))
        .collect()
}

fn parse_param(param: &str) -> Option<RawParam> {
    let param = param.trim();
    if param.is_empty() || param == "void" {
        return None;
    }

    let (without_default, default_value) = split_default_value(param);
    let (type_expr, name) = split_type_and_param_name(without_default.trim())?;

    Some(RawParam {
        name: name.to_string(),
        type_expr: type_expr.to_string(),
        default_value: default_value.map(str::to_string),
    })
}

fn split_default_value(param: &str) -> (&str, Option<&str>) {
    let mut paren_depth = 0usize;
    let mut angle_depth = 0usize;
    for (index, ch) in param.char_indices() {
        match ch {
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '<' => angle_depth += 1,
            '>' => angle_depth = angle_depth.saturating_sub(1),
            '=' if paren_depth == 0 && angle_depth == 0 => {
                return (&param[..index], Some(param[index + ch.len_utf8()..].trim()));
            }
            _ => {}
        }
    }
    (param, None)
}

fn split_type_and_param_name(param: &str) -> Option<(&str, &str)> {
    let name_start = param
        .char_indices()
        .rev()
        .find(|(_, ch)| !(ch.is_ascii_alphanumeric() || *ch == '_'))
        .map_or(0, |(index, ch)| index + ch.len_utf8());
    let name = param[name_start..].trim();
    let type_expr = param[..name_start].trim();
    if name.is_empty() || type_expr.is_empty() {
        None
    } else {
        Some((type_expr, name))
    }
}

fn parse_class_name(source: &str, mut offset: usize) -> Option<(String, usize)> {
    while source[offset..]
        .chars()
        .next()
        .is_some_and(char::is_whitespace)
    {
        offset += source[offset..].chars().next()?.len_utf8();
    }
    let start = offset;
    while source[offset..]
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        offset += source[offset..].chars().next()?.len_utf8();
    }
    if start == offset {
        None
    } else {
        Some((source[start..offset].to_string(), offset))
    }
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

fn collapse_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn split_top_level(source: &str, delimiter: char) -> Vec<String> {
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
            _ if ch == delimiter && paren_depth == 0 && angle_depth == 0 => {
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

fn find_matching_brace(source: &str, open_brace: usize) -> Option<usize> {
    find_matching_delimiter(source, open_brace, b'{', b'}')
}

fn find_matching_paren(source: &str, open_paren: usize) -> Option<usize> {
    find_matching_delimiter(source, open_paren, b'(', b')')
}

fn find_matching_delimiter(
    source: &str,
    open: usize,
    open_byte: u8,
    close_byte: u8,
) -> Option<usize> {
    let mut depth = 0usize;
    for (index, byte) in source.as_bytes().iter().enumerate().skip(open) {
        if *byte == open_byte {
            depth += 1;
        } else if *byte == close_byte {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}

fn is_keyword_boundary(source: &str, start: usize, len: usize) -> bool {
    let before_ok = source[..start]
        .chars()
        .next_back()
        .is_none_or(|ch| !(ch == '_' || ch.is_ascii_alphanumeric()));
    let after_ok = source[start + len..]
        .chars()
        .next()
        .is_none_or(|ch| !(ch == '_' || ch.is_ascii_alphanumeric()));
    before_ok && after_ok
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
    fn extracts_single_line_class_method_declaration() {
        let source = r#"
            class database_api
            {
            public:
               fc::variants get_objects(const vector<object_id_type>& ids) const;
            };
        "#;

        let classes = extract_classes(source, &PathBuf::from("database_api.hpp"));
        assert_eq!(classes.len(), 1);
        assert_eq!(classes[0].name, "database_api");
        assert_eq!(classes[0].methods.len(), 1);

        let method = &classes[0].methods[0];
        assert_eq!(method.name, "get_objects");
        assert_eq!(method.return_type, "fc::variants");
        assert!(method.is_const);
        assert_eq!(method.params.len(), 1);
        assert_eq!(method.params[0].name, "ids");
        assert_eq!(method.params[0].type_expr, "const vector<object_id_type>&");
    }

    #[test]
    fn skips_forward_declaration_before_class_body() {
        let source = r#"
            class database_api_impl;
            class database_api
            {
            public:
               fc::variants get_objects(const vector<object_id_type>& ids) const;
            };
        "#;

        let classes = extract_classes(source, &PathBuf::from("database_api.hpp"));
        assert_eq!(classes.len(), 1);
        assert_eq!(classes[0].name, "database_api");
        assert_eq!(classes[0].methods[0].name, "get_objects");
    }

    #[test]
    fn extracts_multiline_method_with_defaults() {
        let source = r#"
            class history_api
            {
            public:
               vector<operation_history_object> get_account_history(
                  const std::string& account_name_or_id,
                  operation_history_id_type stop = operation_history_id_type(),
                  uint32_t limit = application_options::get_default().api_limit_get_account_history,
                  operation_history_id_type start = operation_history_id_type()
               )const;
            };
        "#;

        let classes = extract_classes(source, &PathBuf::from("api.hpp"));
        let method = &classes[0].methods[0];
        assert_eq!(method.name, "get_account_history");
        assert_eq!(method.return_type, "vector<operation_history_object>");
        assert!(method.is_const);
        assert_eq!(method.params.len(), 4);
        assert_eq!(method.params[0].name, "account_name_or_id");
        assert_eq!(method.params[0].type_expr, "const std::string&");
        assert_eq!(method.params[1].name, "stop");
        assert_eq!(
            method.params[1].default_value.as_deref(),
            Some("operation_history_id_type()")
        );
        assert_eq!(method.params[2].name, "limit");
        assert_eq!(
            method.params[2].default_value.as_deref(),
            Some("application_options::get_default().api_limit_get_account_history")
        );
    }
}
