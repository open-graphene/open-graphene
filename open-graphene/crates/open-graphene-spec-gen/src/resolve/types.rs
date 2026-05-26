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
        if normalize_cpp_type(inner) == "char" {
            return TypeRef::Bytes;
        }
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

    if normalized == "fee_parameters::flat_set_type" {
        return TypeRef::Set {
            inner: Box::new(TypeRef::StaticVariantRef {
                name: "fee_parameters".to_string(),
            }),
            ordering: OrderingRule::StaticVariantTag,
        };
    }

    if normalized == "variant_assert_argument_type" {
        return TypeRef::Pair {
            first: Box::new(TypeRef::Int64 {
                json: None,
                fc: None,
            }),
            second: Box::new(TypeRef::Vector {
                inner: Box::new(TypeRef::Ref {
                    name: "restriction".to_string(),
                }),
            }),
        };
    }

    if let Some((key, value)) = unwrap_two_arg_template(&normalized, "flat_map") {
        return TypeRef::FlatMap {
            key: Box::new(resolve_cpp_type(&key)),
            value: Box::new(resolve_cpp_type(&value)),
            ordering: OrderingRule::Unresolved,
        };
    }

    if let Some((key, value)) = unwrap_two_arg_template(&normalized, "map")
        .or_else(|| unwrap_two_arg_template(&normalized, "std::map"))
    {
        return TypeRef::Map {
            key: Box::new(resolve_cpp_type(&key)),
            value: Box::new(resolve_cpp_type(&value)),
            ordering: OrderingRule::Unresolved,
        };
    }

    if let Some((first, second)) = unwrap_two_arg_template(&normalized, "pair")
        .or_else(|| unwrap_two_arg_template(&normalized, "std::pair"))
    {
        return TypeRef::Pair {
            first: Box::new(resolve_cpp_type(&first)),
            second: Box::new(resolve_cpp_type(&second)),
        };
    }

    if let Some(inner) = unwrap_template(&normalized, "extension") {
        let inner_name = normalized_name(inner);
        if matches!(inner_name.as_str(), "ext" | "additional_options_type") {
            return TypeRef::AnyJson {
                reason: Some(format!(
                    "nested extension payload type requires qualified nested struct extraction: {inner_name}"
                )),
                source: None,
            };
        }
        return TypeRef::Ref { name: inner_name };
    }

    if let Some(inner) = unwrap_template(&normalized, "std::shared_ptr")
        .or_else(|| unwrap_template(&normalized, "shared_ptr"))
    {
        return TypeRef::Ref {
            name: normalized_name(inner),
        };
    }

    match normalized.as_str() {
        "void" => TypeRef::Void,
        "bool" => TypeRef::Bool,
        "char" => TypeRef::String,
        "uint8_t" => TypeRef::Uint8,
        "uint16_t" | "weight_type" => TypeRef::Uint16,
        "uint32_t" | "unsigned" | "unsigned int" | "unsigned_int" => TypeRef::Uint32,
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
        "fc::uint128_t" => TypeRef::Uint128 {
            json: Some(open_graphene_json_schema::types::JsonShape::DecimalString),
        },
        "string" | "std::string" => TypeRef::String,
        "share_type" => TypeRef::Int64 {
            json: None,
            fc: None,
        },
        "public_key_type" => TypeRef::PublicKey {
            chain_prefix: None,
            prefix_ref: Some("chain.publicKeyPrefix".to_string()),
        },
        "address" => TypeRef::Address,
        "additional_asset_options_t" => TypeRef::Ref {
            name: "additional_asset_options".to_string(),
        },
        "range_proof_type" => TypeRef::Bytes,
        "blind_factor_type" => TypeRef::FixedBytes { bytes: 32 },
        "block_id_type" | "checksum_type" => TypeRef::FixedBytes { bytes: 20 },
        "digest_type" => TypeRef::FixedBytes { bytes: 32 },
        "signature_type" => TypeRef::Signature,
        "extensions_type" => TypeRef::StaticVariantRef {
            name: "future_extensions".to_string(),
        },
        "asset"
        | "authority"
        | "account_options"
        | "asset_options"
        | "bitasset_options"
        | "blind_input"
        | "blind_output"
        | "block_header"
        | "chain_parameters"
        | "custom_authority_options_type"
        | "generic_operation_result"
        | "generic_exchange_operation_result"
        | "htlc_options"
        | "memo_data"
        | "maybe_signed_block_header"
        | "no_special_authority"
        | "op_wrapper"
        | "price"
        | "price_feed"
        | "processed_transaction"
        | "restriction"
        | "signed_transaction"
        | "signed_block"
        | "signed_block_header"
        | "stealth_confirmation"
        | "top_holders_special_authority"
        | "transaction"
        | "void_result" => TypeRef::Ref {
            name: normalized.to_string(),
        },
        "extendable_operation_result" => TypeRef::Ref {
            name: "extendable_operation_result_dtl".to_string(),
        },
        "time_point_sec" | "fc::time_point_sec" => TypeRef::TimePointSec,
        "time_point" | "fc::time_point" => TypeRef::TimePoint,
        "operation" => TypeRef::StaticVariantRef {
            name: "operation".to_string(),
        },
        "operation_result" => TypeRef::StaticVariantRef {
            name: "operation_result".to_string(),
        },
        "predicate" => TypeRef::StaticVariantRef {
            name: "predicate".to_string(),
        },
        "htlc_hash" => TypeRef::StaticVariantRef {
            name: "htlc_hash".to_string(),
        },
        "future_extensions" => TypeRef::StaticVariantRef {
            name: "future_extensions".to_string(),
        },
        "argument_type" => TypeRef::StaticVariantRef {
            name: "argument_type".to_string(),
        },
        "vesting_policy_initializer" => TypeRef::StaticVariantRef {
            name: "vesting_policy_initializer".to_string(),
        },
        "worker_initializer" => TypeRef::StaticVariantRef {
            name: "worker_initializer".to_string(),
        },
        "limit_order_auto_action" => TypeRef::StaticVariantRef {
            name: "limit_order_auto_action".to_string(),
        },
        "special_authority" => TypeRef::StaticVariantRef {
            name: "special_authority".to_string(),
        },
        "void_t" => TypeRef::Void,
        "object_id_type" => TypeRef::ObjectId,
        "fc::ecc::commitment_type" => TypeRef::FixedBytes { bytes: 33 },
        "fc::sha1"
        | "fc::ripemd160"
        | "fc::hash160"
        | "htlc_algo_sha1"
        | "htlc_algo_ripemd160"
        | "htlc_algo_hash160" => TypeRef::FixedBytes { bytes: 20 },
        "fc::sha256" | "htlc_algo_sha256" => TypeRef::FixedBytes { bytes: 32 },
        "fc::variants" | "fc::variant" | "fc::variant_object" => TypeRef::AnyJson {
            reason: Some(format!("unstructured Graphene variant type: {normalized}")),
            source: None,
        },
        _ if normalized.ends_with("_id_type") => TypeRef::ProtocolObjectId {
            object_type: normalized.trim_end_matches("_id_type").to_string(),
        },
        _ if normalized.ends_with("_operation")
            || normalized.ends_with("_initializer")
            || normalized.ends_with("_action")
            || normalized.ends_with("_predicate") =>
        {
            TypeRef::Ref {
                name: normalized.to_string(),
            }
        }
        _ if normalized.ends_with("_object") => TypeRef::Ref {
            name: normalized.to_string(),
        },
        _ => TypeRef::AnyJson {
            reason: Some(format!("unsupported C++ type mapping: {normalized}")),
            source: None,
        },
    }
}

fn normalize_cpp_type(type_expr: &str) -> String {
    let mut normalized = type_expr.trim().replace(":: ", "::");
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

fn normalized_name(type_expr: &str) -> String {
    normalize_cpp_type(type_expr).replace(' ', "")
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

fn unwrap_two_arg_template(value: &str, template_name: &str) -> Option<(String, String)> {
    let inner = unwrap_template(value, template_name)?;
    let args = split_top_level_commas(inner);
    if args.len() == 2 {
        Some((args[0].clone(), args[1].clone()))
    } else {
        None
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
    fn maps_two_arg_templates() {
        assert_eq!(
            resolve_cpp_type("flat_map<asset_id_type, price>"),
            TypeRef::FlatMap {
                key: Box::new(TypeRef::ProtocolObjectId {
                    object_type: "asset".to_string()
                }),
                value: Box::new(TypeRef::Ref {
                    name: "price".to_string()
                }),
                ordering: OrderingRule::Unresolved,
            }
        );
        assert_eq!(
            resolve_cpp_type("pair<account_id_type, share_type>"),
            TypeRef::Pair {
                first: Box::new(TypeRef::ProtocolObjectId {
                    object_type: "account".to_string()
                }),
                second: Box::new(TypeRef::Int64 {
                    json: None,
                    fc: None
                }),
            }
        );
    }

    #[test]
    fn maps_authority_field_types() {
        assert_eq!(resolve_cpp_type("weight_type"), TypeRef::Uint16);
        assert_eq!(resolve_cpp_type("address"), TypeRef::Address);
        assert_eq!(
            resolve_cpp_type("flat_map<account_id_type,weight_type>"),
            TypeRef::FlatMap {
                key: Box::new(TypeRef::ProtocolObjectId {
                    object_type: "account".to_string()
                }),
                value: Box::new(TypeRef::Uint16),
                ordering: OrderingRule::Unresolved,
            }
        );
        assert_eq!(
            resolve_cpp_type("flat_map<public_key_type,weight_type>"),
            TypeRef::FlatMap {
                key: Box::new(TypeRef::PublicKey {
                    chain_prefix: None,
                    prefix_ref: Some("chain.publicKeyPrefix".to_string())
                }),
                value: Box::new(TypeRef::Uint16),
                ordering: OrderingRule::Unresolved,
            }
        );
        assert_eq!(
            resolve_cpp_type("flat_map<address,weight_type>"),
            TypeRef::FlatMap {
                key: Box::new(TypeRef::Address),
                value: Box::new(TypeRef::Uint16),
                ordering: OrderingRule::Unresolved,
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
    fn maps_graphene_static_variant_and_time_types() {
        assert_eq!(
            resolve_cpp_type("operation"),
            TypeRef::StaticVariantRef {
                name: "operation".to_string()
            }
        );
        assert_eq!(
            resolve_cpp_type("operation_result"),
            TypeRef::StaticVariantRef {
                name: "operation_result".to_string()
            }
        );
        assert_eq!(resolve_cpp_type("time_point_sec"), TypeRef::TimePointSec);
        assert_eq!(
            resolve_cpp_type("extensions_type"),
            TypeRef::StaticVariantRef {
                name: "future_extensions".to_string()
            }
        );
        assert_eq!(resolve_cpp_type("void_t"), TypeRef::Void);
        assert_eq!(
            resolve_cpp_type("worker_initializer"),
            TypeRef::StaticVariantRef {
                name: "worker_initializer".to_string()
            }
        );
    }

    #[test]
    fn maps_operation_result_arm_types() {
        assert_eq!(
            resolve_cpp_type("void_result"),
            TypeRef::Ref {
                name: "void_result".to_string()
            }
        );
        assert_eq!(
            resolve_cpp_type("asset"),
            TypeRef::Ref {
                name: "asset".to_string()
            }
        );
        assert_eq!(
            resolve_cpp_type("generic_operation_result"),
            TypeRef::Ref {
                name: "generic_operation_result".to_string()
            }
        );
        assert_eq!(
            resolve_cpp_type("extendable_operation_result"),
            TypeRef::Ref {
                name: "extendable_operation_result_dtl".to_string()
            }
        );
    }

    #[test]
    fn maps_common_named_protocol_types() {
        assert_eq!(
            resolve_cpp_type("authority"),
            TypeRef::Ref {
                name: "authority".to_string()
            }
        );
        assert_eq!(
            resolve_cpp_type("public_key_type"),
            TypeRef::PublicKey {
                chain_prefix: None,
                prefix_ref: Some("chain.publicKeyPrefix".to_string())
            }
        );
        assert_eq!(
            resolve_cpp_type("std::shared_ptr<const fee_schedule>"),
            TypeRef::Ref {
                name: "fee_schedule".to_string()
            }
        );
        assert_eq!(
            resolve_cpp_type("variant_assert_argument_type"),
            TypeRef::Pair {
                first: Box::new(TypeRef::Int64 {
                    json: None,
                    fc: None
                }),
                second: Box::new(TypeRef::Vector {
                    inner: Box::new(TypeRef::Ref {
                        name: "restriction".to_string()
                    })
                })
            }
        );
        assert_eq!(
            resolve_cpp_type("fee_parameters::flat_set_type"),
            TypeRef::Set {
                inner: Box::new(TypeRef::StaticVariantRef {
                    name: "fee_parameters".to_string()
                }),
                ordering: OrderingRule::StaticVariantTag
            }
        );
        assert_eq!(
            resolve_cpp_type("fc::ecc::commitment_type"),
            TypeRef::FixedBytes { bytes: 33 }
        );
        assert_eq!(
            resolve_cpp_type("fc::sha256"),
            TypeRef::FixedBytes { bytes: 32 }
        );
        assert_eq!(
            resolve_cpp_type("fc::sha1"),
            TypeRef::FixedBytes { bytes: 20 }
        );
        assert_eq!(
            resolve_cpp_type("htlc_algo_sha256"),
            TypeRef::FixedBytes { bytes: 32 }
        );
        assert_eq!(
            resolve_cpp_type("account_name_eq_lit_predicate"),
            TypeRef::Ref {
                name: "account_name_eq_lit_predicate".to_string()
            }
        );
        assert_eq!(
            resolve_cpp_type("predicate"),
            TypeRef::StaticVariantRef {
                name: "predicate".to_string()
            }
        );
        assert_eq!(
            resolve_cpp_type("htlc_hash"),
            TypeRef::StaticVariantRef {
                name: "htlc_hash".to_string()
            }
        );
        assert_eq!(
            resolve_cpp_type("special_authority"),
            TypeRef::StaticVariantRef {
                name: "special_authority".to_string()
            }
        );
        assert_eq!(
            resolve_cpp_type("top_holders_special_authority"),
            TypeRef::Ref {
                name: "top_holders_special_authority".to_string()
            }
        );
        assert_eq!(
            resolve_cpp_type("htlc_options"),
            TypeRef::Ref {
                name: "htlc_options".to_string()
            }
        );
        assert_eq!(
            resolve_cpp_type("custom_authority_options_type"),
            TypeRef::Ref {
                name: "custom_authority_options_type".to_string()
            }
        );
        assert_eq!(
            resolve_cpp_type("linear_vesting_policy_initializer"),
            TypeRef::Ref {
                name: "linear_vesting_policy_initializer".to_string()
            }
        );
        assert_eq!(resolve_cpp_type("unsigned_int"), TypeRef::Uint32);
        assert_eq!(
            resolve_cpp_type("fc::uint128_t"),
            TypeRef::Uint128 {
                json: Some(open_graphene_json_schema::types::JsonShape::DecimalString)
            }
        );
        assert_eq!(resolve_cpp_type("char"), TypeRef::String);
        assert_eq!(resolve_cpp_type("vector<char>"), TypeRef::Bytes);
        assert_eq!(resolve_cpp_type("std::vector<char>"), TypeRef::Bytes);
        assert_eq!(
            resolve_cpp_type("extension< ext >"),
            TypeRef::AnyJson {
                reason: Some(
                    "nested extension payload type requires qualified nested struct extraction: ext"
                        .to_string()
                ),
                source: None
            }
        );
        assert_eq!(
            resolve_cpp_type("additional_asset_options_t"),
            TypeRef::Ref {
                name: "additional_asset_options".to_string()
            }
        );
        assert_eq!(resolve_cpp_type("range_proof_type"), TypeRef::Bytes);
        assert_eq!(
            resolve_cpp_type("blind_factor_type"),
            TypeRef::FixedBytes { bytes: 32 }
        );
        assert_eq!(
            resolve_cpp_type("block_id_type"),
            TypeRef::FixedBytes { bytes: 20 }
        );
        assert_eq!(
            resolve_cpp_type("digest_type"),
            TypeRef::FixedBytes { bytes: 32 }
        );
        assert_eq!(resolve_cpp_type("signature_type"), TypeRef::Signature);
        assert_eq!(
            resolve_cpp_type("signed_block"),
            TypeRef::Ref {
                name: "signed_block".to_string()
            }
        );
        assert_eq!(
            resolve_cpp_type("processed_transaction"),
            TypeRef::Ref {
                name: "processed_transaction".to_string()
            }
        );
        assert_eq!(
            resolve_cpp_type("transaction"),
            TypeRef::Ref {
                name: "transaction".to_string()
            }
        );
    }

    #[test]
    fn maps_operation_struct_aliases_to_refs() {
        assert_eq!(
            resolve_cpp_type("transfer_operation"),
            TypeRef::Ref {
                name: "transfer_operation".to_string()
            }
        );
    }

    #[test]
    fn maps_object_struct_aliases_to_refs() {
        assert_eq!(
            resolve_cpp_type("operation_history_object"),
            TypeRef::Ref {
                name: "operation_history_object".to_string()
            }
        );
        assert_eq!(
            resolve_cpp_type("vector<operation_history_object>"),
            TypeRef::Vector {
                inner: Box::new(TypeRef::Ref {
                    name: "operation_history_object".to_string()
                })
            }
        );
    }

    #[test]
    fn maps_uint32() {
        assert_eq!(resolve_cpp_type("uint32_t"), TypeRef::Uint32);
    }
}
