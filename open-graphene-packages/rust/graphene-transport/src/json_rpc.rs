use serde_json::{Value, json};

use crate::{CallbackId, TransportError};

pub const GRAPHENE_CALL_METHOD: &str = "call";

#[derive(Clone, Debug, PartialEq)]
pub struct JsonRpcRequest {
    pub id: u64,
    pub method: String,
    pub params: Value,
}

impl JsonRpcRequest {
    pub fn graphene_call(id: u64, api_id: u64, method: &str, params: Value) -> Self {
        Self {
            id,
            method: GRAPHENE_CALL_METHOD.to_string(),
            params: graphene_call_params(api_id, method, params),
        }
    }

    pub fn to_value(&self) -> Value {
        json!({
            "id": self.id,
            "method": self.method,
            "params": self.params,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum JsonRpcInbound {
    Response {
        id: u64,
        result: Value,
    },
    Error {
        id: u64,
        error: Value,
    },
    Notice {
        callback_id: CallbackId,
        payload: Value,
    },
}

pub fn graphene_call_params(api_id: u64, method: &str, params: Value) -> Value {
    json!([api_id, method, params])
}

/// Read a wire integer that may arrive as a JSON number or as a decimal string.
///
/// fc renders an integer above `u32::MAX` as a JSON *string*, so whether a given
/// value arrives as a number or as text depends on the value, not on the field.
/// Measured against a live node: a callback id of 4294967297 (`2^32 + 1`) is
/// echoed in notices as `"4294967297"`.
///
/// This is the same rule the generated bindings apply to chain object fields
/// through `open_graphene_core::deserialize_u64_from_number_or_decimal_string`.
/// It is restated here rather than shared because inbound wire parsing belongs
/// to the transport (D048) and the transport stays free of chain crates (D047).
///
/// Anything else — a float, a non-numeric string, a bool, null — is still
/// rejected, so parsing stays fail-closed.
fn wire_u64(value: &Value) -> Option<u64> {
    match value {
        Value::Number(number) => number.as_u64(),
        Value::String(text) => text.trim().parse().ok(),
        _ => None,
    }
}

pub fn parse_inbound(value: &Value) -> Result<JsonRpcInbound, TransportError> {
    let object = value.as_object().ok_or(TransportError::MessageNotObject)?;

    if object.get("method").and_then(Value::as_str) == Some("notice") {
        return parse_notice(value);
    }

    let id = object
        .get("id")
        .and_then(wire_u64)
        .ok_or(TransportError::ResponseMissingId)?;

    let result = object.get("result");
    let error = object.get("error");

    match (result, error) {
        (Some(_), Some(_)) => Err(TransportError::ResponseHasResultAndError { id }),
        (Some(result), None) => Ok(JsonRpcInbound::Response {
            id,
            result: result.clone(),
        }),
        (None, Some(error)) => Ok(JsonRpcInbound::Error {
            id,
            error: error.clone(),
        }),
        (None, None) => Err(TransportError::ResponseMissingResultOrError { id }),
    }
}

fn parse_notice(value: &Value) -> Result<JsonRpcInbound, TransportError> {
    let params = value
        .get("params")
        .and_then(Value::as_array)
        .ok_or(TransportError::NoticeMissingParams)?;
    if params.len() < 2 {
        return Err(TransportError::NoticeMalformedParams);
    }
    let callback_id = wire_u64(&params[0]).ok_or(TransportError::NoticeInvalidCallbackId)?;
    Ok(JsonRpcInbound::Notice {
        callback_id: CallbackId::new(callback_id),
        payload: params[1].clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_graphene_call_request() {
        let request = JsonRpcRequest::graphene_call(1, 0, "get_objects", json!([["2.1.0"]]));

        assert_eq!(
            request.to_value(),
            json!({
                "id": 1,
                "method": "call",
                "params": [0, "get_objects", [["2.1.0"]]],
            })
        );
    }

    #[test]
    fn builds_graphene_call_params() {
        assert_eq!(
            graphene_call_params(1, "database", json!([])),
            json!([1, "database", []])
        );
    }

    #[test]
    fn parses_response() {
        assert_eq!(
            parse_inbound(&json!({ "id": 1, "result": 123 })).unwrap(),
            JsonRpcInbound::Response {
                id: 1,
                result: json!(123),
            }
        );
    }

    #[test]
    fn parses_error() {
        assert_eq!(
            parse_inbound(&json!({ "id": 1, "error": { "message": "boom" } })).unwrap(),
            JsonRpcInbound::Error {
                id: 1,
                error: json!({ "message": "boom" }),
            }
        );
    }

    #[test]
    fn parses_notice() {
        assert_eq!(
            parse_inbound(&json!({ "method": "notice", "params": [7, [["update"]]] })).unwrap(),
            JsonRpcInbound::Notice {
                callback_id: CallbackId::new(7),
                payload: json!([["update"]]),
            }
        );
    }

    #[test]
    fn rejects_malformed_notice() {
        assert!(matches!(
            parse_inbound(&json!({ "method": "notice", "params": [] })),
            Err(TransportError::NoticeMalformedParams)
        ));
    }

    #[test]
    fn parses_notice_with_callback_id_as_decimal_string() {
        // fc sends any integer above u32::MAX as a string, so the id a
        // subscription registers comes back quoted. Rejecting this shape made
        // every session-mode subscription silently deliver nothing.
        assert_eq!(
            parse_inbound(&json!({ "method": "notice", "params": ["7", [["update"]]] })).unwrap(),
            JsonRpcInbound::Notice {
                callback_id: CallbackId::new(7),
                payload: json!([["update"]]),
            }
        );
    }

    #[test]
    fn parses_notice_with_reserved_range_callback_id() {
        // The exact shape observed on a live node: SUBSCRIPTION_CALLBACK_ID_BASE
        // is 2^32, so every subscription id is past the point where fc quotes it.
        assert_eq!(
            parse_inbound(&json!({
                "method": "notice",
                "params": ["4294967297", [["update"]]],
            }))
            .unwrap(),
            JsonRpcInbound::Notice {
                callback_id: CallbackId::new(4_294_967_297),
                payload: json!([["update"]]),
            }
        );
    }

    #[test]
    fn rejects_notice_with_non_numeric_callback_id() {
        for id in [
            json!("boom"),
            json!(7.5),
            json!(true),
            json!(null),
            json!(-1),
        ] {
            assert!(
                matches!(
                    parse_inbound(&json!({ "method": "notice", "params": [id, []] })),
                    Err(TransportError::NoticeInvalidCallbackId)
                ),
                "expected rejection"
            );
        }
    }

    #[test]
    fn parses_response_with_id_as_decimal_string() {
        // Same rule on the response side: the id is whatever we sent, and a
        // large one comes back quoted.
        assert_eq!(
            parse_inbound(&json!({ "id": "4294967297", "result": 123 })).unwrap(),
            JsonRpcInbound::Response {
                id: 4_294_967_297,
                result: json!(123),
            }
        );
    }

    #[test]
    fn rejects_response_without_id() {
        assert!(matches!(
            parse_inbound(&json!({ "result": 123 })),
            Err(TransportError::ResponseMissingId)
        ));
    }

    #[test]
    fn rejects_response_without_result_or_error() {
        assert!(matches!(
            parse_inbound(&json!({ "id": 1 })),
            Err(TransportError::ResponseMissingResultOrError { id: 1 })
        ));
    }

    #[test]
    fn rejects_response_with_result_and_error() {
        assert!(matches!(
            parse_inbound(&json!({ "id": 1, "result": 123, "error": { "message": "boom" } })),
            Err(TransportError::ResponseHasResultAndError { id: 1 })
        ));
    }
}
