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
}
