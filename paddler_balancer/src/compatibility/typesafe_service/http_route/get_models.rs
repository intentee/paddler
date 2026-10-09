use actix_web::HttpResponse;
use actix_web::web;
use actix_web::web::get;

use paddler_messaging::agent_desired_state::AgentDesiredState;
use paddler_messaging::agent_inference_settings::AgentInferenceSettings;
use paddler_messaging::inference_mode::InferenceMode;

use crate::cluster_serves_another_inference_mode::ClusterServesAnotherInferenceMode;
use crate::compatibility::compatibility_app_data::CompatibilityAppData;
use crate::compatibility::typesafe_service::inference_mode_refusal_http_response::inference_mode_refusal_http_response;
use crate::compatibility::typesafe_service::typesafe_api_path::TypeSafeApiPath;
use crate::compatibility::typesafe_service::typesafe_models::TypeSafeModels;

async fn respond(app_data: web::Data<CompatibilityAppData>) -> HttpResponse {
    let AgentDesiredState {
        inference_settings,
        model,
        ..
    } = app_data
        .balancer_applicable_state_holder
        .get_agent_desired_state();

    match inference_settings {
        AgentInferenceSettings::Decision(_) => HttpResponse::Ok().json(TypeSafeModels::from(model)),
        served_inference_settings @ (AgentInferenceSettings::Embeddings(_)
        | AgentInferenceSettings::TextGeneration(_)) => {
            inference_mode_refusal_http_response(ClusterServesAnotherInferenceMode {
                requested_inference_mode: InferenceMode::Decision,
                served_inference_mode: served_inference_settings.inference_mode(),
            })
        }
    }
}

pub fn get_models(cfg: &mut web::ServiceConfig) {
    cfg.route(TypeSafeApiPath::MODELS, get().to(respond));
}
