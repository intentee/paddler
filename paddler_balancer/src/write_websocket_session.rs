use std::pin::pin;

use actix_ws::Closed;
use actix_ws::Session;
use futures_util::sink::unfold;
use log::debug;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio_util::sync::CancellationToken;

use paddler_frame_writer::frame_writing_end::FrameWritingEnd;
use paddler_frame_writer::write_frames_until_connection_closes::write_frames_until_connection_closes;

use crate::websocket_outbound_frame::WebSocketOutboundFrame;

async fn send_frame(session: &mut Session, frame: WebSocketOutboundFrame) -> Result<(), Closed> {
    match frame {
        WebSocketOutboundFrame::Close(close_reason) => session.clone().close(close_reason).await,
        WebSocketOutboundFrame::Ping => session.ping(b"").await,
        WebSocketOutboundFrame::Pong(payload) => session.pong(&payload).await,
        WebSocketOutboundFrame::Text(text) => session.text(text).await,
    }
}

pub async fn write_websocket_session(
    connection_close: CancellationToken,
    outbound_rx: UnboundedReceiver<WebSocketOutboundFrame>,
    session: Session,
) -> FrameWritingEnd<Closed> {
    let frame_writing_end = write_frames_until_connection_closes(
        connection_close,
        outbound_rx,
        pin!(unfold(session, |mut session, frame| async move {
            send_frame(&mut session, frame).await.map(|()| session)
        })),
    )
    .await;

    match &frame_writing_end {
        FrameWritingEnd::ConnectionClosed => {
            debug!("Abandoning the WebSocket session writes because the peer is not reading them");
        }
        FrameWritingEnd::EveryFrameSent => {}
        FrameWritingEnd::SendFailed(Closed) => {
            debug!("The WebSocket session closed before all of its frames were sent");
        }
    }

    frame_writing_end
}

#[cfg(test)]
mod tests {
    use actix_web::FromRequest as _;
    use actix_web::HttpResponse;
    use actix_web::http::header::CONNECTION;
    use actix_web::http::header::SEC_WEBSOCKET_KEY;
    use actix_web::http::header::SEC_WEBSOCKET_VERSION;
    use actix_web::http::header::UPGRADE;
    use actix_web::test::TestRequest;
    use actix_web::web::Payload;
    use actix_ws::Closed;
    use actix_ws::Session;
    use actix_ws::handle;
    use log::Level;
    use tokio::spawn;
    use tokio::sync::mpsc::unbounded_channel;
    use tokio::task::yield_now;
    use tokio_util::sync::CancellationToken;

    use paddler_capturing_logger::captured_record::CapturedRecord;
    use paddler_capturing_logger::capturing_logger::CapturingLogger;
    use paddler_frame_writer::frame_writing_end::FrameWritingEnd;

    use super::write_websocket_session;
    use crate::websocket_outbound_frame::WebSocketOutboundFrame;

    fn open_websocket_session() -> OpenedWebSocketSession {
        let (http_request, mut request_payload) = TestRequest::default()
            .insert_header((UPGRADE, "websocket"))
            .insert_header((CONNECTION, "upgrade"))
            .insert_header((SEC_WEBSOCKET_VERSION, "13"))
            .insert_header((SEC_WEBSOCKET_KEY, "dGhlIHNhbXBsZSBub25jZQ=="))
            .to_http_parts();
        let payload = Payload::from_request(&http_request, &mut request_payload)
            .into_inner()
            .expect("the test request must carry a payload");
        let (handshake_response, session, _message_stream) =
            handle(&http_request, payload).expect("the test request must upgrade to a WebSocket");

        OpenedWebSocketSession {
            handshake_response,
            session,
        }
    }

    struct OpenedWebSocketSession {
        handshake_response: HttpResponse,
        session: Session,
    }

    #[tokio::test]
    async fn a_writer_abandons_the_writes_its_peer_does_not_read_once_the_connection_closes() {
        let capturing_logger = CapturingLogger::install();

        let OpenedWebSocketSession {
            handshake_response: _unread_handshake_response,
            session,
        } = open_websocket_session();
        let connection_close = CancellationToken::new();
        let (outbound_tx, outbound_rx) = unbounded_channel();
        let writer = spawn(write_websocket_session(
            connection_close.clone(),
            outbound_rx,
            session,
        ));

        connection_close.cancel();

        while !writer.is_finished() {
            outbound_tx
                .send(WebSocketOutboundFrame::Ping)
                .expect("the writer must still receive frames");
            yield_now().await;
        }

        assert!(matches!(
            writer.await.expect("the writer task must not panic"),
            FrameWritingEnd::ConnectionClosed
        ));
        assert!(
            capturing_logger
                .records_for("paddler_balancer::write_websocket_session")
                .iter()
                .any(|CapturedRecord { level, .. }| *level == Level::Debug)
        );
    }

    #[tokio::test]
    async fn a_writer_stops_once_its_session_closes() {
        let OpenedWebSocketSession {
            handshake_response,
            session,
        } = open_websocket_session();
        let (outbound_tx, outbound_rx) = unbounded_channel();
        let writer = spawn(write_websocket_session(
            CancellationToken::new(),
            outbound_rx,
            session,
        ));

        drop(handshake_response);
        outbound_tx
            .send(WebSocketOutboundFrame::Ping)
            .expect("the writer must still receive frames");

        assert!(matches!(
            writer.await.expect("the writer task must not panic"),
            FrameWritingEnd::SendFailed(Closed)
        ));
    }
}
