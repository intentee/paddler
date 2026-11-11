use actix_web::HttpResponse;
use actix_web::web;
use actix_web::web::get;

use paddler_messaging::request_params::decide_params::DecideParams;

use crate::agent_streaming_request::AgentStreamingRequest as _;
use crate::compatibility::compatibility_app_data::CompatibilityAppData;
use crate::compatibility::typesafe_service::typesafe_decision_failure::TypeSafeDecisionFailure;
use crate::compatibility::typesafe_service::typesafe_models::TypeSafeModels;
use crate::constants::TYPESAFE_API_PATH_MODELS;

async fn respond(app_data: web::Data<CompatibilityAppData>) -> HttpResponse {
    match app_data
        .balancer_applicable_state_holder
        .require_decisions()
    {
        Ok(()) => HttpResponse::Ok().json(TypeSafeModels::from(
            app_data
                .agent_controller_pool
                .model_paths_serving(DecideParams::INFERENCE_CAPABILITY),
        )),
        Err(cluster_serves_no_decisions) => {
            TypeSafeDecisionFailure::from(cluster_serves_no_decisions).into_http_response()
        }
    }
}

pub fn get_models(cfg: &mut web::ServiceConfig) {
    cfg.route(TYPESAFE_API_PATH_MODELS, get().to(respond));
}
