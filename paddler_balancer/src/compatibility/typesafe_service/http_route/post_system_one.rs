use actix_web::HttpResponse;
use actix_web::http::StatusCode;
use actix_web::web;
use actix_web::web::post;

use paddler_messaging::request_params::decide_params::DecideParams;
use paddler_typesafe_translation::system_one_request::SystemOneRequest;
use paddler_typesafe_translation::translated_system_one_request::TranslatedSystemOneRequest;

use crate::agent_streaming_request::AgentStreamingRequest as _;
use crate::compatibility::compatibility_app_data::CompatibilityAppData;
use crate::compatibility::typesafe_service::inference_mode_refusal_http_response::inference_mode_refusal_http_response;
use crate::compatibility::typesafe_service::system_one_http_response::system_one_http_response;
use crate::compatibility::typesafe_service::typesafe_api_path::TypeSafeApiPath;
use crate::compatibility::typesafe_service::typesafe_error_body::TypeSafeErrorBody;
use crate::compatibility::typesafe_service::typesafe_json_config::typesafe_json_config;

async fn respond(
    app_data: web::Data<CompatibilityAppData>,
    system_one_request: web::Json<SystemOneRequest>,
) -> HttpResponse {
    if let Err(cluster_serves_another_inference_mode) = app_data
        .balancer_applicable_state_holder
        .require_inference_mode(DecideParams::INFERENCE_MODE)
    {
        return inference_mode_refusal_http_response(cluster_serves_another_inference_mode);
    }

    match system_one_request.into_inner().translate() {
        Ok(TranslatedSystemOneRequest {
            decide_params,
            response_layout,
        }) => {
            let decision_results = app_data.agent_result_stream(decide_params);

            system_one_http_response(&response_layout, decision_results).await
        }
        Err(translation_error) => TypeSafeErrorBody {
            detail: translation_error.to_string(),
        }
        .into_http_response(StatusCode::UNPROCESSABLE_ENTITY),
    }
}

pub fn post_system_one(cfg: &mut web::ServiceConfig) {
    cfg.app_data(typesafe_json_config())
        .route(TypeSafeApiPath::SYSTEM_ONE, post().to(respond));
}
