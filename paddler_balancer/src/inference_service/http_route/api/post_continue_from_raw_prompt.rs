use actix_web::Error;
use actix_web::Responder;
use actix_web::post;
use actix_web::web;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;

use crate::chunk_forwarding_session_controller::identity_transformer::IdentityTransformer;
use crate::http_stream_from_agent::http_stream_from_agent;
use crate::inference_service::app_data::AppData;
use crate::require_token_generation_enabled::require_token_generation_enabled;

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(respond);
}

#[post("/api/v1/continue_from_raw_prompt")]
async fn respond(
    app_data: web::Data<AppData>,
    params: web::Json<ContinueFromRawPromptParams>,
) -> Result<impl Responder, Error> {
    require_token_generation_enabled(&app_data.balancer_applicable_state_holder)?;

    Ok(http_stream_from_agent(
        app_data.buffered_request_manager.clone(),
        app_data.inference_service_configuration.clone(),
        params.into_inner(),
        IdentityTransformer::new(),
        app_data.shutdown.clone(),
    ))
}
