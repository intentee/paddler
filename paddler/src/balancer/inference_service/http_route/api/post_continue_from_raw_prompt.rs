use actix_web::Error;
use actix_web::Responder;
use actix_web::post;
use actix_web::web;
use paddler_types::request_params::ContinueFromRawPromptParams;

use crate::balancer::chunk_forwarding_session_controller::identity_transformer::IdentityTransformer;
use crate::balancer::http_stream_from_agent::http_stream_from_agent;
use crate::balancer::inference_service::app_data::AppData;

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(respond);
}

#[post("/api/v1/continue_from_raw_prompt")]
async fn respond(
    app_data: web::Data<AppData>,
    params: web::Json<ContinueFromRawPromptParams>,
) -> Result<impl Responder, Error> {
    http_stream_from_agent(
        app_data.buffered_request_manager.clone(),
        app_data.inference_service_configuration.clone(),
        params.into_inner(),
        IdentityTransformer::new(),
    )
}
