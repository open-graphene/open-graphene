//! Shared lexical helpers for scanning the vendored C++ sources.
//!
//! All helpers preserve newlines (and therefore byte-offset-to-line mapping)
//! so extraction diagnostics can report positions in the original file.

/// Strips `//` and `/* */` comments, preserving newlines.
///
/// Preprocessor directives are kept: some extractors (e.g. static variants)
/// parse `#define` bodies. Use
/// [`strip_comments_and_directives_preserving_newlines`] where directives
/// should be dropped as well.
pub(crate) fn strip_comments_preserving_newlines(source: &str) -> String {
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

/// Like [`strip_comments_preserving_newlines`], but also strips whole
/// preprocessor directives, following `\` line continuations.
pub(crate) fn strip_comments_and_directives_preserving_newlines(source: &str) -> String {
    let mut output = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    let mut at_line_start = true;

    while let Some(ch) = chars.next() {
        if at_line_start && ch.is_whitespace() && ch != '\n' {
            output.push(ch);
            continue;
        }
        if at_line_start && ch == '#' {
            let mut continued = false;
            for directive_ch in chars.by_ref() {
                if directive_ch == '\\' {
                    continued = true;
                } else if directive_ch == '\n' {
                    output.push('\n');
                    at_line_start = true;
                    if continued {
                        continued = false;
                        continue;
                    }
                    break;
                } else if !directive_ch.is_whitespace() {
                    continued = false;
                }
            }
        } else if ch == '/' && chars.peek() == Some(&'/') {
            chars.next();
            for comment_ch in chars.by_ref() {
                if comment_ch == '\n' {
                    output.push('\n');
                    at_line_start = true;
                    break;
                }
            }
        } else if ch == '/' && chars.peek() == Some(&'*') {
            chars.next();
            let mut previous = '\0';
            for comment_ch in chars.by_ref() {
                if comment_ch == '\n' {
                    output.push('\n');
                    at_line_start = true;
                }
                if previous == '*' && comment_ch == '/' {
                    break;
                }
                previous = comment_ch;
            }
        } else {
            output.push(ch);
            at_line_start = ch == '\n';
        }
    }

    output
}

/// Splits on commas that sit outside any parentheses or angle brackets.
/// Segments are trimmed; only an empty trailing segment is dropped.
pub(crate) fn split_top_level_commas(source: &str) -> Vec<String> {
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

/// Byte offset of the delimiter closing the one at `open`, if balanced.
/// Delimiters must be ASCII, so scanning bytes is offset-exact for UTF-8.
pub(crate) fn find_matching_delimiter(
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

pub(crate) fn find_matching_paren(source: &str, open_paren: usize) -> Option<usize> {
    find_matching_delimiter(source, open_paren, b'(', b')')
}

/// 1-based line number of `offset` within `source`.
pub(crate) fn line_number(source: &str, offset: usize) -> usize {
    source[..offset]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1
}
