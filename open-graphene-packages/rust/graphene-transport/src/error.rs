use serde_json::Value;
use std::io;
use std::time::Duration;
use thiserror::Error;

use crate::CallbackId;

#[derive(Debug, Error)]
pub enum TransportError {
    #[error("JSON-RPC message is not an object")]
    MessageNotObject,

    #[error("JSON-RPC response missing integer id")]
    ResponseMissingId,

    #[error("JSON-RPC response {id} contains both result and error")]
    ResponseHasResultAndError { id: u64 },

    #[error("JSON-RPC response {id} missing result or error")]
    ResponseMissingResultOrError { id: u64 },

    #[error("Graphene notice missing params array")]
    NoticeMissingParams,

    #[error("Graphene notice params must contain callback id and payload")]
    NoticeMalformedParams,

    #[error("Graphene notice callback id is not an integer")]
    NoticeInvalidCallbackId,

    #[error("Graphene callback params must be an array")]
    CallbackParamsNotArray,

    #[error("timed out waiting for Graphene callback notice {callback_id} after {timeout:?}")]
    CallbackTimeout {
        callback_id: CallbackId,
        timeout: Duration,
    },

    #[error("unsupported JSON-RPC inbound message")]
    UnsupportedInboundMessage,

    #[error("websocket error: {0}")]
    WebSocket(String),

    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Graphene RPC error for request {id}: {error}")]
    RpcError { id: u64, error: Value },

    #[error("unexpected JSON-RPC response id {actual}; expected {expected}")]
    UnexpectedResponseId { expected: u64, actual: u64 },

    #[error("websocket connection closed")]
    ConnectionClosed,

    #[error("Graphene API `{name}` is unavailable on this node")]
    MissingApi { name: &'static str },

    #[error("Graphene API `{name}` discovery returned non-integer id: {value}")]
    InvalidApiId { name: &'static str, value: Value },

    #[error("database.get_chain_id returned non-string value: {value}")]
    InvalidChainId { value: Value },
}

impl TransportError {
    pub(crate) fn websocket(error: tungstenite::Error) -> Self {
        Self::WebSocket(error.to_string())
    }
}
