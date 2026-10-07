use actix_web::Error;
use actix_web::Responder;
use actix_web::web;
use actix_web::web::post;

use paddler_messaging::api_path::ApiPath;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;

use crate::chunk_forwarding_session_controller::identity_transformer::IdentityTransformer;
use crate::inference_service::app_data::AppData;
use crate::ndjson_response::ndjson_response;
use crate::unbounded_stream_from_agent::unbounded_stream_from_agent;
use crate::unbounded_stream_from_agent_params::UnboundedStreamFromAgentParams;

async fn respond(
    app_data: web::Data<AppData>,
    params: web::Json<ContinueFromRawPromptParams>,
) -> Result<impl Responder, Error> {
    Ok(ndjson_response(unbounded_stream_from_agent(
        UnboundedStreamFromAgentParams {
            buffered_request_manager: app_data.buffered_request_manager.clone(),
            inference_service_configuration: app_data.inference_service_configuration.clone(),
            request_params: params.into_inner(),
            shutdown: app_data.shutdown.clone(),
            transformer: IdentityTransformer::new(),
        },
    )))
}

pub fn post_continue_from_raw_prompt(cfg: &mut web::ServiceConfig) {
    cfg.route(ApiPath::CONTINUE_FROM_RAW_PROMPT, post().to(respond));
}
