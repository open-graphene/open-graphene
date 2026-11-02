use crate::ChainApiError;

pub(super) fn unexpected_response(
    method: &'static str,
    message: impl Into<String>,
) -> ChainApiError {
    ChainApiError::UnexpectedResponse {
        method,
        message: message.into(),
    }
}
