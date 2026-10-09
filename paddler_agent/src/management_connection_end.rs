use tokio_tungstenite::tungstenite::Utf8Bytes;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;

const fn described_close_frame(code: CloseCode, reason: &'static str) -> CloseFrame {
    CloseFrame {
        code,
        reason: Utf8Bytes::from_static(reason),
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum ManagementConnectionEnd {
    AgentShuttingDown,
    BalancerClosedConnection,
    BalancerMessageUndeserializable,
    BalancerReusedInFlightRequestId,
    BalancerSentUnsupportedFrame,
    IncomingStreamFailed,
    OutgoingMessageUnserializable,
    OutgoingStreamFailed,
}

impl ManagementConnectionEnd {
    #[must_use]
    pub const fn close_frame(&self) -> Option<CloseFrame> {
        match self {
            Self::AgentShuttingDown => Some(described_close_frame(
                CloseCode::Away,
                "Agent shutting down",
            )),
            Self::BalancerClosedConnection
            | Self::IncomingStreamFailed
            | Self::OutgoingStreamFailed => None,
            Self::BalancerMessageUndeserializable => Some(described_close_frame(
                CloseCode::Invalid,
                "Balancer message could not be deserialized",
            )),
            Self::BalancerReusedInFlightRequestId => Some(described_close_frame(
                CloseCode::Policy,
                "Balancer reused the id of a request in flight",
            )),
            Self::BalancerSentUnsupportedFrame => Some(described_close_frame(
                CloseCode::Unsupported,
                "Balancer sent a frame that is not text",
            )),
            Self::OutgoingMessageUnserializable => Some(described_close_frame(
                CloseCode::Error,
                "Agent message could not be serialized",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;

    use super::ManagementConnectionEnd;

    #[test]
    fn an_abandoned_connection_tells_the_balancer_why_it_closes() {
        let close_codes: Vec<Option<CloseCode>> = [
            ManagementConnectionEnd::BalancerMessageUndeserializable,
            ManagementConnectionEnd::BalancerReusedInFlightRequestId,
            ManagementConnectionEnd::BalancerSentUnsupportedFrame,
            ManagementConnectionEnd::OutgoingMessageUnserializable,
        ]
        .iter()
        .map(|connection_end| {
            connection_end
                .close_frame()
                .map(|close_frame| close_frame.code)
        })
        .collect();

        assert_eq!(
            close_codes,
            vec![
                Some(CloseCode::Invalid),
                Some(CloseCode::Policy),
                Some(CloseCode::Unsupported),
                Some(CloseCode::Error),
            ]
        );
    }
}
