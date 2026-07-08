use std::path::Path;

use super::facts::{RawObjectType, SourceLoc};
use super::lexer::{
    find_matching_paren, line_number, split_top_level_commas, strip_comments_preserving_newlines,
};

pub fn extract_object_types(source_text: &str, file: &Path) -> Vec<RawObjectType> {
    let source = strip_comments_preserving_newlines(source_text);
    let mut out = Vec::new();
    let mut offset = 0usize;

    while let Some(relative_start) = source[offset..].find("GRAPHENE_DEFINE_IDS") {
        let macro_start = offset + relative_start;
        let Some(open_paren) = source[macro_start..].find('(').map(|idx| macro_start + idx) else {
            offset = macro_start + "GRAPHENE_DEFINE_IDS".len();
            continue;
        };
        let Some(close_paren) = find_matching_paren(&source, open_paren) else {
            offset = open_paren + 1;
            continue;
        };
        let args = split_top_level_commas(&source[open_paren + 1..close_paren]);
        if args.len() == 4 {
            let id_namespace = args[0].trim().to_string();
            let object_space_name = args[1].trim().to_string();
            let object_type_prefix = args[2].trim().to_string();
            let object_space = object_space_number(&object_space_name);
            let names = parse_boost_pp_seq(&args[3]);
            let line = line_number(&source, macro_start);

            for (type_id, object_type) in names.into_iter().enumerate() {
                out.push(RawObjectType {
                    object_type: object_type.clone(),
                    cpp_alias: format!("{object_type}_id_type"),
                    object_space_name: object_space_name.clone(),
                    object_space,
                    object_type_name: format!("{object_type_prefix}{object_type}_object_type"),
                    type_id: Some(type_id as u32),
                    struct_ref: Some(format!("{object_type}_object")),
                    source: SourceLoc {
                        file: file.to_path_buf(),
                        line,
                    },
                    id_namespace: id_namespace.clone(),
                });
            }
        }
        offset = close_paren + 1;
    }

    out
}

fn object_space_number(name: &str) -> Option<u32> {
    match name {
        "protocol_ids" => Some(1),
        "implementation_ids" => Some(2),
        _ => None,
    }
}

fn parse_boost_pp_seq(source: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut offset = 0usize;
    while let Some(relative_open) = source[offset..].find('(') {
        let open = offset + relative_open;
        let Some(close) = source[open + 1..].find(')').map(|idx| open + 1 + idx) else {
            break;
        };
        let name = source[open + 1..close].trim();
        if !name.is_empty() {
            names.push(name.to_string());
        }
        offset = close + 1;
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn extracts_graphene_define_ids_object_types() {
        let source = r#"
            GRAPHENE_DEFINE_IDS(protocol, protocol_ids, /* no prefix */,
                                /* 1.0.x */ (null)
                                /* 1.1.x */ (base)
                                /* 1.2.x */ (account)
                                /* 1.3.x */ (asset)
                               )
        "#;

        let object_types = extract_object_types(source, &PathBuf::from("types.hpp"));

        assert_eq!(object_types.len(), 4);
        assert_eq!(object_types[2].object_type, "account");
        assert_eq!(object_types[2].cpp_alias, "account_id_type");
        assert_eq!(object_types[2].object_space, Some(1));
        assert_eq!(object_types[2].type_id, Some(2));
        assert_eq!(
            object_types[2].struct_ref.as_deref(),
            Some("account_object")
        );
    }
}
