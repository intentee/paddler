use actix_web::HttpResponse;
use actix_web::web;
use actix_web::web::get;

use paddler_messaging::request_params::decide_params::DecideParams;

use crate::agent_streaming_request::AgentStreamingRequest as _;
use crate::compatibility::compatibility_app_data::CompatibilityAppData;
use crate::compatibility::typesafe_service::cluster_serves_no_decisions_http_response::cluster_serves_no_decisions_http_response;
use crate::compatibility::typesafe_service::typesafe_api_path::TypeSafeApiPath;
use crate::compatibility::typesafe_service::typesafe_models::TypeSafeModels;

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
            cluster_serves_no_decisions_http_response(cluster_serves_no_decisions)
        }
    }
}

pub fn get_models(cfg: &mut web::ServiceConfig) {
    cfg.route(TypeSafeApiPath::MODELS, get().to(respond));
}
