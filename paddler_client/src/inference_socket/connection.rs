use std::sync::Arc;

use futures_util::Stream;
use futures_util::StreamExt;
use futures_util::stream::unfold;
use serde::Serialize;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::task::JoinHandle;
use tokio_tungstenite::connect_async;
use tokio_util::sync::CancellationToken;
use url::Url;

use paddler_messaging::inference_client::message::Message as InferenceMessage;

use crate::inference_socket::inbound_message_router::InboundMessageRouter;
use crate::inference_socket::inference_socket_exchange::InferenceSocketExchange;
use crate::inference_socket::outbound_request::OutboundRequest;
use crate::inference_socket::url::url;
use crate::result::Result;

struct StreamState {
    cancellation_token: CancellationToken,
    connection: Arc<Connection>,
    is_complete: bool,
    request_id: String,
    response_rx: UnboundedReceiver<Result<InferenceMessage>>,
}

impl StreamState {
    const fn mark_complete(&mut self) {
        self.is_complete = true;
    }
}

impl Drop for StreamState {
    fn drop(&mut self) {
        if !self.is_complete {
            self.connection.stop_responding_to(&self.request_id);
        }
    }
}

pub struct Connection {
    exchange: Arc<InferenceSocketExchange>,
    read_task: JoinHandle<()>,
    write_task: JoinHandle<()>,
}

impl Connection {
    /// # Errors
    ///
    /// Returns [`Error::InferenceSocketUrlSchemeRejected`](crate::error::Error::InferenceSocketUrlSchemeRejected) when the URL scheme is not http, https, ws or wss, and [`Error::WebSocket`](crate::error::Error::WebSocket) when the WebSocket handshake fails.
    pub async fn connect(connection_url: Url) -> Result<Self> {
        let ws_url = url(connection_url)?;
        let (ws_stream, _) = connect_async(ws_url.as_str()).await?;
        let (ws_write, ws_read) = ws_stream.split();
        let exchange = Arc::new(InferenceSocketExchange::default());

        Ok(Self {
            read_task: InboundMessageRouter {
                exchange: exchange.clone(),
            }
            .spawn_read_task(ws_read),
            write_task: exchange.clone().spawn_write_task(ws_write),
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

    pub fn response_stream(
        self: Arc<Self>,
        cancellation_token: CancellationToken,
        request_id: String,
        response_rx: UnboundedReceiver<Result<InferenceMessage>>,
    ) -> impl Stream<Item = Result<InferenceMessage>> + Send + 'static {
        unfold(
            StreamState {
                cancellation_token,
                connection: self,
                is_complete: false,
                request_id,
                response_rx,
            },
            |mut state| async move {
                let received_message = state
                    .cancellation_token
                    .run_until_cancelled(state.response_rx.recv())
                    .await;

                match received_message {
                    None => None,
                    Some(None) => {
                        state.mark_complete();

                        None
                    }
                    Some(Some(message)) => Some((message, state)),
                }
            },
        )
    }

    /// # Errors
    ///
    /// Returns [`Error::Json`](crate::error::Error::Json) when the message cannot be serialized, [`Error::ConnectionDropped`](crate::error::Error::ConnectionDropped) when the connection is already closed, and [`Error::InferenceRequestIdInFlight`](crate::error::Error::InferenceRequestIdInFlight) when a request with the same id is already in flight.
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
