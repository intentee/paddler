use actix_web::Error;
use actix_web::Responder;
use actix_web::error::ErrorBadRequest;
use actix_web::error::ErrorServiceUnavailable;
use actix_web::web;
use actix_web::web::post;

use paddler_messaging::api_path::ApiPath;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::raw_parameters_schema::RawParametersSchema;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use paddler_messaging::request_params::validates::Validates as _;

use crate::agent_streaming_request::AgentStreamingRequest as _;
use crate::capability_availability::CapabilityAvailability;
use crate::chunk_forwarding_session_controller::identity_transformer::IdentityTransformer;
use crate::inference_service::app_data::AppData;
use crate::invalid_request_parameters_description::invalid_request_parameters_description;
use crate::ndjson_response::ndjson_response;
use crate::unbounded_stream_from_agent::UnboundedStreamFromAgent;

async fn respond(
    app_data: web::Data<AppData>,
    params: web::Json<ContinueFromConversationHistoryParams<RawParametersSchema>>,
) -> Result<impl Responder, Error> {
    if let CapabilityAvailability::Lacking(cluster_lacks_capability) = app_data
        .agent_controller_pool
        .capability_availability(ContinueFromConversationHistoryParams::<ValidatedParametersSchema>::REQUESTED_CAPABILITY)
    {
        return Err(ErrorServiceUnavailable(cluster_lacks_capability));
    }

    let validated_params = params.into_inner().validate().map_err(|validation_error| {
        ErrorBadRequest(invalid_request_parameters_description(&validation_error))
    })?;

    Ok(ndjson_response(
        UnboundedStreamFromAgent {
            buffered_request_manager: app_data.buffered_request_manager.clone(),
            inference_item_timeout: app_data
                .inference_service_configuration
                .inference_item_timeout,
            request_params: validated_params,
            shutdown: app_data.shutdown.clone(),
            transformer: IdentityTransformer::new(),
        }
        .into_stream(),
    ))
}

pub fn post_continue_from_conversation_history(cfg: &mut web::ServiceConfig) {
    cfg.route(
        ApiPath::continue_from_conversation_history(),
        post().to(respond),
    );
}
