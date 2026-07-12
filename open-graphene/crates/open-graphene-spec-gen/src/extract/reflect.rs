use std::path::Path;

use super::facts::{RawReflect, SourceLoc};
use super::lexer::{
    find_matching_paren, line_number, split_top_level_commas, strip_comments_preserving_newlines,
};

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
        let derived = matches!(
            macro_name,
            "FC_REFLECT_DERIVED" | "FC_REFLECT_DERIVED_NO_TYPENAME"
        );
        let field_arg_index = if derived { 2 } else { 1 };
        if let Some(type_name) = args.first() {
            let bases = if derived {
                args.get(1)
                    .map(|bases_arg| parse_reflect_fields(bases_arg))
                    .unwrap_or_default()
            } else {
                vec![]
            };
            // FC_REFLECT_EMPTY reflects a type with no fields at all.
            let fields = args
                .get(field_arg_index)
                .map(|fields_arg| parse_reflect_fields(fields_arg))
                .unwrap_or_default();
            reflects.push(RawReflect {
                type_name: type_name.trim().to_string(),
                bases,
                fields,
                source: SourceLoc {
                    file: file.to_path_buf(),
                    line: line_number(&source, macro_start),
                },
                derived,
            });
        }
        offset = close_paren + 1;
    }

    reflects
}

const REFLECT_MACROS: [&str; 4] = [
    "FC_REFLECT",
    "FC_REFLECT_DERIVED",
    "FC_REFLECT_DERIVED_NO_TYPENAME",
    "FC_REFLECT_EMPTY",
];

fn find_next_reflect_macro(source: &str, offset: usize) -> Option<(usize, &'static str)> {
    REFLECT_MACROS
        .iter()
        .filter_map(|macro_name| {
            source[offset..]
                .find(&format!("{macro_name}("))
                .map(|relative| (offset + relative, *macro_name))
        })
        // Longest name wins on a tie so `FC_REFLECT_DERIVED_NO_TYPENAME(`
        // is not consumed as `FC_REFLECT(` would never match here, but
        // `FC_REFLECT_DERIVED(` vs `FC_REFLECT_DERIVED_NO_TYPENAME(` differ
        // by position anyway; ties cannot happen for distinct suffixes.
        .min_by_key(|(index, _)| *index)
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
        assert_eq!(
            reflects[0].bases,
            vec!["graphene::db::abstract_object<operation_history_object>"]
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
