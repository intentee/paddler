use serde::Serialize;
use serde_json::to_string;

use paddler_messaging::inference_server::message::Message as InferenceServerMessage;
use paddler_messaging::inference_server::notification::Notification as InferenceServerNotification;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;

use crate::error::Error;
use crate::error::Result;

pub struct OutboundRequest {
    pub request_frame: String,
    pub request_id: String,
    pub stop_frame: String,
}

impl OutboundRequest {
    pub fn serialize<TMessage: Serialize>(request_id: String, message: &TMessage) -> Result<Self> {
        to_string(message)
            .and_then(|request_frame| {
                to_string(
                    &InferenceServerMessage::<ValidatedParametersSchema>::Notification(
                        InferenceServerNotification::StopRespondingTo(request_id.clone()),
                    ),
                )
                .map(|stop_frame| Self {
                    request_frame,
                    request_id,
                    stop_frame,
                })
            })
            .map_err(Error::Json)
    }
}
