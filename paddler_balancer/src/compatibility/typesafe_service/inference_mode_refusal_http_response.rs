use actix_web::HttpResponse;

use crate::cluster_serves_another_inference_mode::ClusterServesAnotherInferenceMode;
use crate::compatibility::typesafe_service::typesafe_error_body::TypeSafeErrorBody;
use crate::compatibility::upstream_failure::UpstreamFailure;

#[must_use]
pub fn inference_mode_refusal_http_response(
    cluster_serves_another_inference_mode: ClusterServesAnotherInferenceMode,
) -> HttpResponse {
    TypeSafeErrorBody {
        detail: cluster_serves_another_inference_mode.to_string(),
    }
    .into_http_response(UpstreamFailure::Unavailable.status_code())
}
