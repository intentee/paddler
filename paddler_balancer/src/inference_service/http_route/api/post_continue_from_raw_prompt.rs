use actix_web::Error;
use actix_web::Responder;
use actix_web::web;
use actix_web::web::post;

use paddler_messaging::constants::API_PATH_CONTINUE_FROM_RAW_PROMPT;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;

use crate::chunk_forwarding_session_controller::identity_transformer::IdentityTransformer;
use crate::inference_service::app_data::AppData;
use crate::ndjson_response::ndjson_response;
use crate::unbounded_stream_from_agent::UnboundedStreamFromAgent;

async fn respond(
    app_data: web::Data<AppData>,
    params: web::Json<ContinueFromRawPromptParams>,
) -> Result<impl Responder, Error> {
    Ok(ndjson_response(
        UnboundedStreamFromAgent {
            buffered_request_manager: app_data.buffered_request_manager.clone(),
            inference_item_timeout: app_data
                .inference_service_configuration
                .inference_item_timeout,
            request_params: params.into_inner(),
            shutdown: app_data.shutdown.clone(),
            transformer: IdentityTransformer::new(),
        }
        .into_stream(),
    ))
}

pub fn post_continue_from_raw_prompt(cfg: &mut web::ServiceConfig) {
    cfg.route(API_PATH_CONTINUE_FROM_RAW_PROMPT, post().to(respond));
}
