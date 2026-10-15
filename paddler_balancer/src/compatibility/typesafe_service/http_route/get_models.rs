use actix_web::HttpResponse;
use actix_web::web;
use actix_web::web::get;

use paddler_messaging::inference_mode::InferenceMode;

use crate::compatibility::compatibility_app_data::CompatibilityAppData;
use crate::compatibility::typesafe_service::inference_mode_refusal_http_response::inference_mode_refusal_http_response;
use crate::compatibility::typesafe_service::typesafe_api_path::TypeSafeApiPath;
use crate::compatibility::typesafe_service::typesafe_models::TypeSafeModels;

async fn respond(app_data: web::Data<CompatibilityAppData>) -> HttpResponse {
    match app_data
        .balancer_applicable_state_holder
        .require_inference_mode(InferenceMode::Decision)
    {
        Ok(balancer_applicable_state) => HttpResponse::Ok().json(TypeSafeModels::from(
            balancer_applicable_state.agent_desired_state.model.clone(),
        )),
        Err(cluster_serves_another_inference_mode) => {
            inference_mode_refusal_http_response(cluster_serves_another_inference_mode)
        }
    }
}

pub fn get_models(cfg: &mut web::ServiceConfig) {
    cfg.route(TypeSafeApiPath::MODELS, get().to(respond));
}
