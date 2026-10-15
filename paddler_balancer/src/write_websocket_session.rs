use actix_ws::Closed;
use actix_ws::Session;
use log::debug;
use tokio::select;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio_util::sync::CancellationToken;

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
    mut outbound_rx: UnboundedReceiver<WebSocketOutboundFrame>,
    mut session: Session,
) {
    while let Some(frame) = outbound_rx.recv().await {
        select! {
            biased;
            send_outcome = send_frame(&mut session, frame) => {
                if send_outcome.is_err() {
                    debug!("The WebSocket session closed before all of its frames were sent");

                    return;
                }
            }
            () = connection_close.cancelled() => {
                debug!("Abandoning the WebSocket session writes because the peer is not reading them");

                return;
            }
        }
    }
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
    use actix_ws::Session;
    use actix_ws::handle;
    use tokio::spawn;
    use tokio::sync::mpsc::unbounded_channel;
    use tokio::task::yield_now;
    use tokio_util::sync::CancellationToken;

    use super::write_websocket_session;
    use crate::websocket_outbound_frame::WebSocketOutboundFrame;

    async fn open_websocket_session() -> (HttpResponse, Session) {
        let (http_request, mut request_payload) = TestRequest::default()
            .insert_header((UPGRADE, "websocket"))
            .insert_header((CONNECTION, "upgrade"))
            .insert_header((SEC_WEBSOCKET_VERSION, "13"))
            .insert_header((SEC_WEBSOCKET_KEY, "dGhlIHNhbXBsZSBub25jZQ=="))
            .to_http_parts();
        let payload = Payload::from_request(&http_request, &mut request_payload)
            .await
            .unwrap();
        let (handshake_response, session, _message_stream) =
            handle(&http_request, payload).unwrap();

        (handshake_response, session)
    }

    #[tokio::test]
    async fn a_writer_abandons_the_writes_its_peer_does_not_read_once_the_connection_closes() {
        let (_unread_handshake_response, session) = open_websocket_session().await;
        let connection_close = CancellationToken::new();
        let (outbound_tx, outbound_rx) = unbounded_channel();
        let writer = spawn(write_websocket_session(
            connection_close.clone(),
            outbound_rx,
            session,
        ));

        connection_close.cancel();

        while !writer.is_finished() {
            outbound_tx.send(WebSocketOutboundFrame::Ping).unwrap();
            yield_now().await;
        }

        writer.await.unwrap();
    }

    #[tokio::test]
    async fn a_writer_stops_once_its_session_closes() {
        let (handshake_response, session) = open_websocket_session().await;
        let (outbound_tx, outbound_rx) = unbounded_channel();
        let writer = spawn(write_websocket_session(
            CancellationToken::new(),
            outbound_rx,
            session,
        ));

        drop(handshake_response);
        outbound_tx.send(WebSocketOutboundFrame::Ping).unwrap();

        writer.await.unwrap();
    }
}
