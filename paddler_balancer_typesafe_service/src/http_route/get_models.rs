use actix_web::HttpResponse;
use actix_web::web;
use actix_web::web::get;

use paddler_balancer::agent_streaming_request::AgentStreamingRequest as _;
use paddler_balancer::compatibility::compatibility_app_data::CompatibilityAppData;
use paddler_balancer::decisions_serving::DecisionsServing;
use paddler_messaging::request_params::decide_params::DecideParams;

use crate::constants::TYPESAFE_API_PATH_MODELS;
use crate::typesafe_decision_failure::TypeSafeDecisionFailure;
use crate::typesafe_models::TypeSafeModels;

async fn respond(app_data: web::Data<CompatibilityAppData>) -> HttpResponse {
    match app_data.agent_controller_pool.decisions_serving() {
        DecisionsServing::Served => HttpResponse::Ok().json(TypeSafeModels::from(
            app_data
                .agent_controller_pool
                .model_paths_serving(DecideParams::INFERENCE_CAPABILITY),
        )),
        DecisionsServing::NotServed(cluster_serves_no_decisions) => {
            TypeSafeDecisionFailure::from(cluster_serves_no_decisions).into_http_response()
        }
    }
}

pub fn get_models(cfg: &mut web::ServiceConfig) {
    cfg.route(TYPESAFE_API_PATH_MODELS, get().to(respond));
}
