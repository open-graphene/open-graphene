use crate::SwaplockApiError;

pub(super) fn unexpected_response(
    method: &'static str,
    message: impl Into<String>,
) -> SwaplockApiError {
    SwaplockApiError::UnexpectedResponse {
        method,
        message: message.into(),
    }
}
