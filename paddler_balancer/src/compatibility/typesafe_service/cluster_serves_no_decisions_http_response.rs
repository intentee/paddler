use actix_web::HttpResponse;

use crate::cluster_serves_no_decisions::ClusterServesNoDecisions;
use crate::compatibility::typesafe_service::typesafe_error_body::TypeSafeErrorBody;
use crate::compatibility::upstream_failure::UpstreamFailure;

#[must_use]
pub fn cluster_serves_no_decisions_http_response(
    cluster_serves_no_decisions: ClusterServesNoDecisions,
) -> HttpResponse {
    TypeSafeErrorBody {
        detail: cluster_serves_no_decisions.to_string(),
    }
    .into_http_response(UpstreamFailure::Unavailable.status_code())
}
