use log::error;

use paddler_messaging::inference_client::message::Message as OutgoingMessage;
use paddler_messaging::jsonrpc::error::Error as JsonRpcError;
use paddler_messaging::jsonrpc::error_envelope::ErrorEnvelope;

use crate::controls_session::ControlsSession;

pub async fn respond_with_error<TControlsSession>(
    error: JsonRpcError,
    request_id: String,
    session_controller: &mut TControlsSession,
) where
    TControlsSession: ControlsSession<OutgoingMessage>,
{
    session_controller
        .send_response(OutgoingMessage::Error(ErrorEnvelope {
            request_id: request_id.clone(),
            error,
        }))
        .await
        .unwrap_or_else(|err| {
            error!("Failed to send response for request {request_id:?}: {err}");
        });
}
