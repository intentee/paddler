use serde_json::Error as SerdeJsonError;
use thiserror::Error;

use paddler_messaging::inference_client::message::Message as OutgoingMessage;

#[derive(Debug, Error)]
pub enum AgentRelayError {
    #[error("The socket of agent {agent_id} is closed")]
    AgentSocketClosed { agent_id: String },
    #[error("Unable to serialize a message relayed to the client")]
    MessageUnserializable(#[source] SerdeJsonError),
    #[error("The client's protocol cannot carry this message: {message:?}")]
    MessageNotRelayable { message: Box<OutgoingMessage> },
    #[error("A response is already being relayed under request id {request_id}")]
    RequestIdAlreadyRelayed { request_id: String },
}
