use serde_json::{json, Value};

use crate::TransportError;

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
    Response { id: u64, result: Value },
    Error { id: u64, error: Value },
    Notice { callback_id: u64, payload: Value },
}

pub fn graphene_call_params(api_id: u64, method: &str, params: Value) -> Value {
    json!([api_id, method, params])
}

pub fn parse_inbound(value: &Value) -> Result<JsonRpcInbound, TransportError> {
    let object = value.as_object().ok_or(TransportError::MessageNotObject)?;

    if object.get("method").and_then(Value::as_str) == Some("notice") {
        return parse_notice(value);
    }

    let id = object
        .get("id")
        .and_then(Value::as_u64)
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
    let callback_id = params[0]
        .as_u64()
        .ok_or(TransportError::NoticeInvalidCallbackId)?;
    Ok(JsonRpcInbound::Notice {
        callback_id,
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
                callback_id: 7,
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
    fn rejects_notice_with_non_integer_callback_id() {
        assert!(matches!(
            parse_inbound(&json!({ "method": "notice", "params": ["7", []] })),
            Err(TransportError::NoticeInvalidCallbackId)
        ));
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
