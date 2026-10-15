use actix_web::http::StatusCode;
use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OpenAIErrorType {
    InvalidRequestError,
    ServerError,
}

impl From<StatusCode> for OpenAIErrorType {
    fn from(status_code: StatusCode) -> Self {
        if status_code.is_client_error() {
            Self::InvalidRequestError
        } else {
            Self::ServerError
        }
    }
}
