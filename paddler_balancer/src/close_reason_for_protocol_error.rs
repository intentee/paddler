use actix_ws::CloseCode;
use actix_ws::CloseReason;
use actix_ws::ProtocolError;

use crate::max_websocket_frame_size::MAX_WEBSOCKET_FRAME_SIZE;

#[must_use]
pub fn close_reason_for_protocol_error(protocol_error: &ProtocolError) -> CloseReason {
    match protocol_error {
        ProtocolError::Overflow => CloseReason {
            code: CloseCode::Size,
            description: Some(format!(
                "Message exceeded the maximum allowed frame size of {MAX_WEBSOCKET_FRAME_SIZE} bytes"
            )),
        },
        ProtocolError::Io(io_error) => CloseReason {
            code: CloseCode::Error,
            description: Some(io_error.to_string()),
        },
        protocol_violation => CloseReason {
            code: CloseCode::Protocol,
            description: Some(protocol_violation.to_string()),
        },
    }
}

#[cfg(test)]
mod tests {
    use std::io;

    use actix_ws::CloseCode;
    use actix_ws::ProtocolError;

    use super::close_reason_for_protocol_error;

    #[test]
    fn closes_an_oversized_frame_as_too_big() {
        assert_eq!(
            close_reason_for_protocol_error(&ProtocolError::Overflow).code,
            CloseCode::Size
        );
    }

    #[test]
    fn closes_an_input_failure_as_an_error_carrying_its_cause() {
        let close_reason = close_reason_for_protocol_error(&ProtocolError::Io(io::Error::other(
            "Exceeded maximum continuation size",
        )));

        assert_eq!(close_reason.code, CloseCode::Error);
        assert_eq!(
            close_reason.description.as_deref(),
            Some("Exceeded maximum continuation size")
        );
    }

    #[test]
    fn closes_a_protocol_violation_as_a_protocol_error() {
        assert_eq!(
            close_reason_for_protocol_error(&ProtocolError::UnmaskedFrame).code,
            CloseCode::Protocol
        );
    }
}
