use actix_web::error::InternalError;
use actix_web::http::StatusCode;
use actix_web::web::JsonConfig;

use crate::compatibility::typesafe_service::typesafe_error_body::TypeSafeErrorBody;

#[must_use]
pub fn typesafe_json_config() -> JsonConfig {
    JsonConfig::default().error_handler(|json_payload_error, _request| {
        let unprocessable_entity = TypeSafeErrorBody {
            detail: json_payload_error.to_string(),
        }
        .into_http_response(StatusCode::UNPROCESSABLE_ENTITY);

        InternalError::from_response(json_payload_error, unprocessable_entity).into()
    })
}
