use actix_ws::CloseCode;
use actix_ws::CloseReason;
use actix_ws::ProtocolError;

use crate::max_websocket_message_size::MAX_WEBSOCKET_MESSAGE_SIZE;

const fn described_close_reason(code: CloseCode, description: String) -> CloseReason {
    CloseReason {
        code,
        description: Some(description),
    }
}

#[derive(Debug)]
pub enum WebSocketCloseCause {
    AgentAlreadyRegistered,
    AgentDeregistered,
    AgentMessageUndeserializable,
    AgentStatusBeforeRegistration,
    ConnectionCloseRequested,
    IncomingMessageTooBig,
    IncomingStreamFailed,
    PeerClosedConnection,
    ProtocolViolated,
    ServerShuttingDown,
    SessionAlreadyClosed,
}

impl WebSocketCloseCause {
    #[must_use]
    pub fn close_reason(&self) -> Option<CloseReason> {
        match self {
            Self::AgentAlreadyRegistered => Some(described_close_reason(
                CloseCode::Policy,
                "Agent is already registered".to_owned(),
            )),
            Self::AgentDeregistered
            | Self::ConnectionCloseRequested
            | Self::PeerClosedConnection
            | Self::SessionAlreadyClosed => None,
            Self::AgentMessageUndeserializable => Some(described_close_reason(
                CloseCode::Invalid,
                "Agent message could not be deserialized".to_owned(),
            )),
            Self::AgentStatusBeforeRegistration => Some(described_close_reason(
                CloseCode::Policy,
                "Agent sent its status before registering".to_owned(),
            )),
            Self::IncomingMessageTooBig => Some(described_close_reason(
                CloseCode::Size,
                format!(
                    "Message exceeded the maximum allowed message size of {MAX_WEBSOCKET_MESSAGE_SIZE} bytes"
                ),
            )),
            Self::IncomingStreamFailed => Some(described_close_reason(
                CloseCode::Error,
                "Failed to read the incoming message stream".to_owned(),
            )),
            Self::ProtocolViolated => Some(described_close_reason(
                CloseCode::Protocol,
                "WebSocket protocol violated".to_owned(),
            )),
            Self::ServerShuttingDown => Some(described_close_reason(
                CloseCode::Away,
                "Server shutting down".to_owned(),
            )),
        }
    }
}

impl From<&ProtocolError> for WebSocketCloseCause {
    fn from(protocol_error: &ProtocolError) -> Self {
        match protocol_error {
            ProtocolError::Overflow => Self::IncomingMessageTooBig,
            ProtocolError::Io(_) => Self::IncomingStreamFailed,
            ProtocolError::BadOpCode
            | ProtocolError::ContinuationFragment(_)
            | ProtocolError::ContinuationNotStarted
            | ProtocolError::ContinuationStarted
            | ProtocolError::InvalidLength(_)
            | ProtocolError::InvalidOpcode(_)
            | ProtocolError::MaskedFrame
            | ProtocolError::UnmaskedFrame => Self::ProtocolViolated,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io;

    use actix_ws::CloseCode;
    use actix_ws::CloseReason;
    use actix_ws::ProtocolError;

    use super::WebSocketCloseCause;

    fn close_reason_for(protocol_error: &ProtocolError) -> Option<CloseReason> {
        WebSocketCloseCause::from(protocol_error).close_reason()
    }

    #[test]
    fn closes_an_oversized_frame_as_too_big() {
        assert_eq!(
            close_reason_for(&ProtocolError::Overflow),
            Some(CloseReason {
                code: CloseCode::Size,
                description: Some(
                    "Message exceeded the maximum allowed message size of 52428800 bytes"
                        .to_owned()
                ),
            })
        );
    }

    #[test]
    fn closes_an_input_failure_as_an_error_with_a_fixed_description() {
        assert_eq!(
            close_reason_for(&ProtocolError::Io(io::Error::other(
                "Exceeded maximum continuation size"
            ))),
            Some(CloseReason {
                code: CloseCode::Error,
                description: Some("Failed to read the incoming message stream".to_owned()),
            })
        );
    }

    #[test]
    fn closes_a_protocol_violation_as_a_protocol_error() {
        assert_eq!(
            close_reason_for(&ProtocolError::UnmaskedFrame),
            Some(CloseReason {
                code: CloseCode::Protocol,
                description: Some("WebSocket protocol violated".to_owned()),
            })
        );
    }
}
