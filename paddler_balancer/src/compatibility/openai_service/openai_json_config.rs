use actix_web::error::InternalError;
use actix_web::http::StatusCode;
use actix_web::web::JsonConfig;

use crate::compatibility::openai_service::openai_error_body::OpenAIErrorBody;
use crate::compatibility::openai_service::openai_error_type::OpenAIErrorType;

#[must_use]
pub fn openai_json_config() -> JsonConfig {
    JsonConfig::default().error_handler(|json_payload_error, _request| {
        let bad_request = OpenAIErrorBody {
            error_type: OpenAIErrorType::InvalidRequestError,
            message: json_payload_error.to_string(),
        }
        .into_http_response(StatusCode::BAD_REQUEST);

        InternalError::from_response(json_payload_error, bad_request).into()
    })
}
