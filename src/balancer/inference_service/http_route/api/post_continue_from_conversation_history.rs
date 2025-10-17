use actix_web::Error;
use actix_web::Responder;
use actix_web::error::ErrorBadRequest;
use actix_web::post;
use actix_web::web;

use crate::validates::Validates as _;
use crate::balancer::inference_service::app_data::AppData;
use crate::request_params::ContinueFromConversationHistoryParams;
use crate::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::raw_parameters_schema::RawParametersSchema;
use crate::balancer::http_stream_from_agent::http_stream_from_agent;
use crate::balancer::chunk_forwarding_session_controller::identity_transformer::IdentityTransformer;

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(respond);
}

#[post("/api/v1/continue_from_conversation_history")]
async fn respond(
    app_data: web::Data<AppData>,
    params: web::Json<ContinueFromConversationHistoryParams<RawParametersSchema>>,
) -> Result<impl Responder, Error> {
    http_stream_from_agent(
        app_data.buffered_request_manager.clone(),
        app_data.inference_service_configuration.clone(),
        match params.into_inner().validate() {
            Ok(validated_params) => validated_params,
            Err(validation_error) => {
                return Err(ErrorBadRequest(format!(
                    "Invalid request parameters: {validation_error}"
                )));
            }
        },
        IdentityTransformer::new(),
    )
}
