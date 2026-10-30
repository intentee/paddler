use std::sync::Arc;

use futures_util::StreamExt;
use serde::Serialize;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::task::JoinHandle;
use tokio_tungstenite::connect_async;
use url::Url;

use paddler_messaging::inference_client::message::Message as InferenceMessage;

use crate::error::Result;
use crate::inference_socket::inbound_message_router::InboundMessageRouter;
use crate::inference_socket::inference_socket_exchange::InferenceSocketExchange;
use crate::inference_socket::outbound_request::OutboundRequest;
use crate::inference_socket::spawn_read_task::spawn_read_task;
use crate::inference_socket::spawn_write_task::spawn_write_task;
use crate::inference_socket::url::url;

pub struct Connection {
    exchange: Arc<InferenceSocketExchange>,
    read_task: JoinHandle<()>,
    write_task: JoinHandle<()>,
}

impl Connection {
    pub async fn connect(connection_url: Url) -> Result<Self> {
        let ws_url = url(connection_url)?;
        let (ws_stream, _) = connect_async(ws_url.as_str()).await?;
        let (ws_write, ws_read) = ws_stream.split();
        let exchange = Arc::new(InferenceSocketExchange::default());

        Ok(Self {
            read_task: spawn_read_task(
                ws_read,
                InboundMessageRouter {
                    exchange: exchange.clone(),
                },
            ),
            write_task: spawn_write_task(ws_write, exchange.clone()),
            exchange,
        })
    }

    pub async fn disconnected(&self) {
        self.exchange.closed().await;
    }

    #[must_use]
    pub fn is_disconnected(&self) -> bool {
        self.exchange.is_closed()
    }

    pub fn send<TMessage: Serialize>(
        &self,
        request_id: String,
        message: &TMessage,
    ) -> Result<UnboundedReceiver<Result<InferenceMessage>>> {
        OutboundRequest::serialize(request_id, message)
            .and_then(|outbound_request| self.exchange.register(outbound_request))
    }

    pub fn stop_responding_to(&self, request_id: &str) {
        self.exchange.stop_responding_to(request_id);
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        self.read_task.abort();
        self.write_task.abort();
    }
}

#[cfg(test)]
mod tests {
    use std::io::ErrorKind;

    use tokio::io::AsyncReadExt as _;
    use tokio::io::AsyncWriteExt as _;
    use tokio::join;
    use tokio::net::TcpListener;
    use tokio_tungstenite::tungstenite::Error as WebSocketError;
    use url::Url;

    use super::Connection;
    use crate::error::Error;

    const PLAINTEXT_HTTP_REPLY_START: &[u8] = b"HTTP/";
    const TLS_HANDSHAKE_RECORD_CONTENT_TYPE: u8 = 22;

    #[tokio::test]
    async fn connect_rejects_a_url_that_cannot_reach_an_inference_socket() {
        assert!(matches!(
            Connection::connect(
                Url::parse("ftp://127.0.0.1/models").expect("the test URL must be valid"),
            )
            .await,
            Err(Error::InferenceSocketUrlSchemeRejected { scheme, .. }) if scheme == "ftp"
        ));
    }

    #[tokio::test]
    async fn connect_fails_for_an_unreachable_server() {
        assert!(matches!(
            Connection::connect(
                Url::parse("http://127.0.0.1:1").expect("the test URL must be valid"),
            )
            .await,
            Err(Error::WebSocket(WebSocketError::Io(io_error)))
                if io_error.kind() == ErrorKind::ConnectionRefused
        ));
    }

    #[tokio::test]
    async fn connect_opens_a_tls_handshake_with_an_https_balancer() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("the test listener must bind");
        let listener_address = listener
            .local_addr()
            .expect("the test listener must have an address");
        let plaintext_server = async move {
            let (mut accepted_stream, _peer_address) =
                listener.accept().await.expect("the client must connect");

            accepted_stream
                .write_all(PLAINTEXT_HTTP_REPLY_START)
                .await
                .expect("the plaintext reply must reach the client");

            accepted_stream
        };
        let connection = Connection::connect(
            Url::parse(&format!("https://{listener_address}")).expect("the test URL must be valid"),
        );

        let (mut accepted_stream, connection_outcome) = join!(plaintext_server, connection);

        assert_eq!(
            accepted_stream
                .read_u8()
                .await
                .expect("the client must have sent a record"),
            TLS_HANDSHAKE_RECORD_CONTENT_TYPE
        );
        assert_eq!(
            connection_outcome
                .err()
                .map(|connect_error| connect_error.to_string()),
            Some(
                "WebSocket error: IO error: received corrupt message of type InvalidContentType"
                    .to_owned()
            )
        );
    }
}
