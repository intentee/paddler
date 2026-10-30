use actix_web::HttpResponse;
use actix_web::http::StatusCode;

use paddler_typesafe_translation::typesafe_translation_error::TypeSafeTranslationError;

use crate::compatibility::typesafe_service::typesafe_error_body::TypeSafeErrorBody;
use crate::compatibility::upstream_failure::UpstreamFailure;
use crate::relay_failure::RelayFailure;

#[derive(Debug, Eq, PartialEq)]
pub struct TypeSafeDecisionFailure {
    pub detail: String,
    pub status_code: StatusCode,
}

impl TypeSafeDecisionFailure {
    #[must_use]
    pub fn into_http_response(self) -> HttpResponse {
        TypeSafeErrorBody {
            detail: self.detail,
        }
        .into_http_response(self.status_code)
    }
}

impl From<RelayFailure> for TypeSafeDecisionFailure {
    fn from(relay_failure: RelayFailure) -> Self {
        Self {
            detail: relay_failure.description(),
            status_code: UpstreamFailure::from(&relay_failure).status_code(),
        }
    }
}

impl From<TypeSafeTranslationError> for TypeSafeDecisionFailure {
    fn from(translation_error: TypeSafeTranslationError) -> Self {
        Self {
            detail: translation_error.to_string(),
            status_code: UpstreamFailure::RelayFailed.status_code(),
        }
    }
}
