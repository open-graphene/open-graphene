use open_graphene_json_schema::types::{OrderingRule, TypeRef};

pub fn resolve_cpp_type(type_expr: &str) -> TypeRef {
    let normalized = normalize_cpp_type(type_expr);

    if let Some(inner) = unwrap_template(&normalized, "optional")
        .or_else(|| unwrap_template(&normalized, "fc::optional"))
    {
        return TypeRef::Optional {
            inner: Box::new(resolve_cpp_type(inner)),
        };
    }

    if let Some(inner) = unwrap_template(&normalized, "vector")
        .or_else(|| unwrap_template(&normalized, "std::vector"))
    {
        return TypeRef::Vector {
            inner: Box::new(resolve_cpp_type(inner)),
        };
    }

    if let Some(inner) = unwrap_template(&normalized, "flat_set")
        .or_else(|| unwrap_template(&normalized, "std::set"))
    {
        return TypeRef::Set {
            inner: Box::new(resolve_cpp_type(inner)),
            ordering: OrderingRule::Unresolved,
        };
    }

    match normalized.as_str() {
        "void" => TypeRef::Void,
        "bool" => TypeRef::Bool,
        "uint8_t" => TypeRef::Uint8,
        "uint16_t" => TypeRef::Uint16,
        "uint32_t" | "unsigned" | "unsigned int" => TypeRef::Uint32,
        "int32_t" | "int" => TypeRef::Int32 {
            fc: None,
            source: None,
        },
        "int64_t" => TypeRef::Int64 {
            json: None,
            fc: None,
        },
        "uint64_t" => TypeRef::Uint64 {
            json: None,
            fc: None,
        },
        "string" | "std::string" => TypeRef::String,
        "object_id_type" => TypeRef::ObjectId,
        "fc::variants" | "fc::variant" | "fc::variant_object" => TypeRef::AnyJson {
            reason: Some(format!("unstructured Graphene variant type: {normalized}")),
            source: None,
        },
        _ if normalized.ends_with("_id_type") => TypeRef::ProtocolObjectId {
            object_type: normalized.trim_end_matches("_id_type").to_string(),
        },
        _ => TypeRef::AnyJson {
            reason: Some(format!("unsupported C++ type mapping: {normalized}")),
            source: None,
        },
    }
}

fn normalize_cpp_type(type_expr: &str) -> String {
    let mut normalized = type_expr.trim().to_string();
    loop {
        let next = normalized
            .trim()
            .trim_start_matches("const ")
            .trim_end_matches('&')
            .trim_end_matches('*')
            .trim()
            .to_string();
        if next == normalized {
            break;
        }
        normalized = next;
    }
    collapse_whitespace(&normalized)
}

fn collapse_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn unwrap_template<'a>(value: &'a str, template_name: &str) -> Option<&'a str> {
    let prefix = format!("{template_name}<");
    let rest = value.strip_prefix(&prefix)?;
    if !rest.ends_with('>') {
        return None;
    }
    Some(rest[..rest.len() - 1].trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_const_string_reference_to_string() {
        assert_eq!(resolve_cpp_type("const std::string&"), TypeRef::String);
    }

    #[test]
    fn maps_optional_bool() {
        assert_eq!(
            resolve_cpp_type("optional<bool>"),
            TypeRef::Optional {
                inner: Box::new(TypeRef::Bool)
            }
        );
    }

    #[test]
    fn maps_vector_object_id_reference() {
        assert_eq!(
            resolve_cpp_type("const vector<object_id_type>&"),
            TypeRef::Vector {
                inner: Box::new(TypeRef::ObjectId)
            }
        );
    }

    #[test]
    fn maps_protocol_object_id_aliases() {
        assert_eq!(
            resolve_cpp_type("operation_history_id_type"),
            TypeRef::ProtocolObjectId {
                object_type: "operation_history".to_string()
            }
        );
    }

    #[test]
    fn maps_uint32() {
        assert_eq!(resolve_cpp_type("uint32_t"), TypeRef::Uint32);
    }
}
