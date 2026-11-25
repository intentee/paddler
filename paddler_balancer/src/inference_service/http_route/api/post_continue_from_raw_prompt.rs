use actix_web::Error;
use actix_web::Responder;
use actix_web::error::ErrorServiceUnavailable;
use actix_web::web;
use actix_web::web::post;

use paddler_messaging::api_path::ApiPath;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;

use crate::agent_streaming_request::AgentStreamingRequest as _;
use crate::capability_availability::CapabilityAvailability;
use crate::chunk_forwarding_session_controller::identity_transformer::IdentityTransformer;
use crate::inference_service::app_data::AppData;
use crate::ndjson_response::ndjson_response;
use crate::unbounded_stream_from_agent::UnboundedStreamFromAgent;

async fn respond(
    app_data: web::Data<AppData>,
    params: web::Json<ContinueFromRawPromptParams>,
) -> Result<impl Responder, Error> {
    if let CapabilityAvailability::Lacking(cluster_lacks_capability) = app_data
        .agent_controller_pool
        .capability_availability(ContinueFromRawPromptParams::REQUESTED_CAPABILITY)
    {
        return Err(ErrorServiceUnavailable(cluster_lacks_capability));
    }

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
    cfg.route(ApiPath::continue_from_raw_prompt(), post().to(respond));
}
