use serde_json::Value;
use thiserror::Error;

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

    #[error("unsupported JSON-RPC inbound message")]
    UnsupportedInboundMessage,

    #[error("websocket error: {0}")]
    WebSocket(String),

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Graphene RPC error for request {id}: {error}")]
    RpcError { id: u64, error: Value },

    #[error("unexpected JSON-RPC response id {actual}; expected {expected}")]
    UnexpectedResponseId { expected: u64, actual: u64 },

    #[error("websocket connection closed")]
    ConnectionClosed,
}

impl TransportError {
    pub(crate) fn websocket(error: tungstenite::Error) -> Self {
        Self::WebSocket(error.to_string())
    }
}
