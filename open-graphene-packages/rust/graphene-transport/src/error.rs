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

    #[error("timed out waiting for Graphene response after {timeout:?}")]
    ResponseTimeout { timeout: Duration },

    #[error("Graphene live dispatcher stopped")]
    DispatcherStopped,

    #[error(
        "subscription callback id {callback_id} is below the reserved base {base}; allocate ids with LiveTransportHandle::allocate_callback_id so they cannot collide with request ids"
    )]
    SubscriptionCallbackIdReserved { callback_id: CallbackId, base: u64 },

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

    #[error("at least one RPC server is required")]
    MissingServers,

    #[error("all RPC servers failed: {attempts:?}")]
    AllServersFailed {
        attempts: Vec<crate::ServerConnectFailure>,
    },

    #[error("connected RPC server returned unexpected chain id: {mismatch:?}")]
    ChainIdMismatch { mismatch: crate::ChainIdMismatch },
}

impl TransportError {
    pub(crate) fn websocket(error: tungstenite::Error) -> Self {
        Self::WebSocket(error.to_string())
    }

    /// A single frame the node sent could not be understood. The connection
    /// itself is intact, so long-lived consumers (the live dispatcher) skip
    /// the frame instead of tearing everything down.
    pub fn is_malformed_frame(&self) -> bool {
        matches!(
            self,
            Self::MessageNotObject
                | Self::ResponseMissingId
                | Self::ResponseHasResultAndError { .. }
                | Self::ResponseMissingResultOrError { .. }
                | Self::NoticeMissingParams
                | Self::NoticeMalformedParams
                | Self::NoticeInvalidCallbackId
                | Self::UnsupportedInboundMessage
                | Self::Json(_)
        )
    }
}
