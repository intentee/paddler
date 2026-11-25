use actix_web::Error;
use actix_web::Responder;
use actix_web::error::ErrorBadRequest;
use actix_web::web;
use actix_web::web::post;

use paddler_messaging::constants::API_PATH_CONTINUE_FROM_CONVERSATION_HISTORY;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::raw_parameters_schema::RawParametersSchema;
use paddler_messaging::validates::Validates as _;

use crate::chunk_forwarding_session_controller::identity_transformer::IdentityTransformer;
use crate::inference_service::app_data::AppData;
use crate::invalid_request_parameters_description::invalid_request_parameters_description;
use crate::ndjson_response::ndjson_response;
use crate::unbounded_stream_from_agent::UnboundedStreamFromAgent;

async fn respond(
    app_data: web::Data<AppData>,
    params: web::Json<ContinueFromConversationHistoryParams<RawParametersSchema>>,
) -> Result<impl Responder, Error> {
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
        API_PATH_CONTINUE_FROM_CONVERSATION_HISTORY,
        post().to(respond),
    );
}
