use std::num::NonZeroUsize;
use std::sync::Arc;

use paddler_messaging::inference_client::message::Message as InferenceMessage;
use paddler_messaging::inference_client::notification::Notification;
use serde::Serialize;
use serde_json::to_string;
use tokio::sync::Mutex;
use tokio::sync::broadcast;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio_util::sync::CancellationToken;
use url::Url;

use crate::error::Error;
use crate::error::Result;
use crate::inference_message_stream::InferenceMessageStream;
use crate::inference_socket::connection::Connection;
use crate::inference_socket::response_stream::response_stream;

struct EstablishedRequest {
    connection: Arc<Connection>,
    response_rx: UnboundedReceiver<Result<InferenceMessage>>,
}

pub struct Pool {
    url: Url,
    connections: Mutex<Vec<Option<Arc<Connection>>>>,
    capacity: NonZeroUsize,
    next_idx: Mutex<usize>,
    notification_tx: broadcast::Sender<Notification>,
}

impl Pool {
    pub fn new(url: Url, capacity: NonZeroUsize) -> Self {
        let (notification_tx, _initial_notification_rx) = broadcast::channel(capacity.get());

        Self {
            url,
            connections: Mutex::new((0..capacity.get()).map(|_| None).collect()),
            capacity,
            next_idx: Mutex::new(0),
            notification_tx,
        }
    }

    pub fn subscribe_to_notifications(&self) -> broadcast::Receiver<Notification> {
        self.notification_tx.subscribe()
    }

    pub async fn send_request<TMessage: Serialize>(
        &self,
        cancellation_token: CancellationToken,
        request_id: String,
        message: TMessage,
    ) -> Result<InferenceMessageStream> {
        let json = to_string(&message)?;
        let conn_idx = self.next_connection_index().await;

        let Some(established_request_result) = cancellation_token
            .run_until_cancelled(self.establish_request(conn_idx, json, request_id.clone()))
            .await
        else {
            return Err(Error::InferenceRequestCancelled { request_id });
        };

        let EstablishedRequest {
            connection,
            response_rx,
        } = established_request_result?;

        Ok(Box::pin(response_stream(
            cancellation_token,
            connection,
            request_id,
            response_rx,
        )))
    }

    async fn establish_request(
        &self,
        conn_idx: usize,
        json: String,
        request_id: String,
    ) -> Result<EstablishedRequest> {
        self.ensure_connection(conn_idx).await?;

        let connection = self.get_connection(conn_idx).await?;

        match connection.send(request_id.clone(), json.clone()) {
            Ok(response_rx) => Ok(EstablishedRequest {
                connection,
                response_rx,
            }),
            Err(Error::ConnectionDropped { .. }) => {
                self.ensure_connection(conn_idx).await?;

                let reconnected_connection = self.get_connection(conn_idx).await?;
                let reconnected_request_id = request_id.clone();

                match reconnected_connection.send(request_id, json) {
                    Ok(response_rx) => Ok(EstablishedRequest {
                        connection: reconnected_connection,
                        response_rx,
                    }),
                    Err(reconnection_error) => Err(Error::ReconnectionFailed {
                        request_id: reconnected_request_id,
                        source: Box::new(reconnection_error),
                    }),
                }
            }
            Err(other_error) => Err(other_error),
        }
    }

    async fn get_connection(&self, index: usize) -> Result<Arc<Connection>> {
        let connections = self.connections.lock().await;

        connections[index].clone().ok_or(Error::ConnectionSlotEmpty)
    }

    async fn next_connection_index(&self) -> usize {
        let mut idx = self.next_idx.lock().await;
        let conn_idx = *idx % self.capacity.get();
        *idx = idx.wrapping_add(1);

        conn_idx
    }

    async fn ensure_connection(&self, index: usize) -> Result<()> {
        let needs_connect = {
            let connections = self.connections.lock().await;

            connections[index]
                .as_ref()
                .is_none_or(|connection| connection.is_disconnected())
        };

        if needs_connect {
            let new_connection =
                Connection::connect(self.url.clone(), self.notification_tx.clone()).await?;
            let mut connections = self.connections.lock().await;
            connections[index] = Some(Arc::new(new_connection));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;
    use std::time::Duration;

    use futures_util::StreamExt as _;
    use tokio::net::TcpListener;
    use tokio::task::JoinHandle;
    use tokio::time::timeout;
    use tokio_tungstenite::accept_async;
    use tokio_util::sync::CancellationToken;
    use url::Url;

    use super::Pool;
    use crate::error::Error;
    use crate::error::Result;

    const SHUTDOWN_DEADLINE: Duration = Duration::from_secs(5);
    const TEST_REQUEST_ID: &str = "test-request";

    struct PoolConnectedToFixture {
        pool: Pool,
        server_task: JoinHandle<()>,
    }

    impl PoolConnectedToFixture {
        async fn connect() -> Self {
            let listener = TcpListener::bind("127.0.0.1:0")
                .await
                .expect("the fixture server must bind a loopback port");
            let address = listener
                .local_addr()
                .expect("the fixture server must report its address");

            let server_task = tokio::spawn(async move {
                let (tcp_stream, _peer_address) = listener
                    .accept()
                    .await
                    .expect("the fixture server must accept the connection");
                let mut server_websocket = accept_async(tcp_stream)
                    .await
                    .expect("the fixture server must complete the websocket handshake");

                while server_websocket.next().await.is_some() {}
            });

            let url =
                Url::parse(&format!("http://{address}")).expect("the fixture URL must be valid");
            let pool = Pool::new(url, NonZeroUsize::MIN);

            pool.ensure_connection(0)
                .await
                .expect("the pool must connect to the fixture server");

            Self { pool, server_task }
        }

        async fn inject_send_errors(&self, errors: Vec<Error>) {
            self.pool
                .get_connection(0)
                .await
                .expect("the pool must hold the fixture connection")
                .inject_send_errors(errors);
        }

        async fn send_request(&self) -> Result<()> {
            self.pool
                .send_request(
                    CancellationToken::new(),
                    TEST_REQUEST_ID.to_owned(),
                    "test-message",
                )
                .await
                .map(|_established_stream| ())
        }

        async fn shut_down(self) {
            drop(self.pool);

            timeout(SHUTDOWN_DEADLINE, self.server_task)
                .await
                .expect("the fixture server must observe the closed connection")
                .expect("the fixture server task must not panic");
        }
    }

    fn dropped_connection_error() -> Error {
        Error::ConnectionDropped {
            request_id: TEST_REQUEST_ID.to_owned(),
        }
    }

    #[tokio::test]
    async fn round_robins_across_connection_slots() {
        let url = Url::parse("http://127.0.0.1:1").expect("the test URL must be valid");
        let capacity = NonZeroUsize::new(3).expect("3 is not zero");
        let pool = Pool::new(url, capacity);

        assert_eq!(pool.next_connection_index().await, 0);
        assert_eq!(pool.next_connection_index().await, 1);
        assert_eq!(pool.next_connection_index().await, 2);
        assert_eq!(pool.next_connection_index().await, 0);
    }

    #[tokio::test]
    async fn establishes_a_request_on_a_healthy_connection() {
        let fixture = PoolConnectedToFixture::connect().await;

        assert!(
            fixture.send_request().await.is_ok(),
            "a healthy connection must establish the request without retrying"
        );

        fixture.shut_down().await;
    }

    #[tokio::test]
    async fn retries_a_request_whose_send_failed_on_a_dropped_connection() {
        let fixture = PoolConnectedToFixture::connect().await;

        fixture
            .inject_send_errors(vec![dropped_connection_error()])
            .await;

        assert!(
            fixture.send_request().await.is_ok(),
            "a dropped connection must be re-established and the request resent"
        );

        fixture.shut_down().await;
    }

    #[tokio::test]
    async fn reports_reconnection_failed_when_the_resent_request_also_fails() {
        let fixture = PoolConnectedToFixture::connect().await;

        fixture
            .inject_send_errors(vec![dropped_connection_error(), dropped_connection_error()])
            .await;

        assert!(
            matches!(
                fixture.send_request().await,
                Err(Error::ReconnectionFailed { .. })
            ),
            "a resend that fails again must report the reconnection failure"
        );

        fixture.shut_down().await;
    }

    #[tokio::test]
    async fn does_not_retry_a_send_error_other_than_a_dropped_connection() {
        let fixture = PoolConnectedToFixture::connect().await;

        fixture
            .inject_send_errors(vec![Error::ConnectionSlotEmpty])
            .await;

        assert!(
            matches!(
                fixture.send_request().await,
                Err(Error::ConnectionSlotEmpty)
            ),
            "an error that is not a dropped connection must surface without a retry"
        );

        fixture.shut_down().await;
    }
}
