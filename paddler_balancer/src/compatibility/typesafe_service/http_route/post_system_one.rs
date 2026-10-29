use actix_web::HttpResponse;
use actix_web::http::StatusCode;
use actix_web::web;
use actix_web::web::post;

use paddler_typesafe_translation::system_one_request::SystemOneRequest;
use paddler_typesafe_translation::translated_system_one_request::TranslatedSystemOneRequest;

use crate::compatibility::compatibility_app_data::CompatibilityAppData;
use crate::compatibility::typesafe_service::cluster_serves_no_decisions_http_response::cluster_serves_no_decisions_http_response;
use crate::compatibility::typesafe_service::system_one_http_response::system_one_http_response;
use crate::compatibility::typesafe_service::typesafe_api_path::TypeSafeApiPath;
use crate::compatibility::typesafe_service::typesafe_error_body::TypeSafeErrorBody;
use crate::compatibility::typesafe_service::typesafe_json_config::typesafe_json_config;

async fn respond(
    app_data: web::Data<CompatibilityAppData>,
    system_one_request: web::Json<SystemOneRequest>,
) -> HttpResponse {
    if let Err(cluster_serves_no_decisions) = app_data
        .balancer_applicable_state_holder
        .require_decisions()
    {
        return cluster_serves_no_decisions_http_response(cluster_serves_no_decisions);
    }

    match system_one_request.into_inner().translate() {
        Ok(TranslatedSystemOneRequest {
            decide_params,
            response_layout,
        }) => system_one_http_response(&response_layout, app_data.agent_relay(decide_params)).await,
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
