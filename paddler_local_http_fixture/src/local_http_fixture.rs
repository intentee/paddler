use std::io;
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::sync::Arc;

use std::io::ErrorKind;

use tokio::net::TcpListener;
use tokio::select;
use tokio::spawn;
use tokio::sync::oneshot;
use tokio::sync::watch::error::RecvError;
use tokio::task::JoinError;
use tokio::task::JoinHandle;

use crate::fixture_response::FixtureResponse;
use crate::observed_requests::ObservedRequests;

async fn serve_sequentially_until_stopped(
    listener: TcpListener,
    response: FixtureResponse,
    observed_requests: Arc<ObservedRequests>,
    mut serving_stopped: oneshot::Receiver<()>,
) -> io::Result<()> {
    loop {
        let (stream, _peer_addr) = select! {
            biased;
            accepted_connection = listener.accept() => accepted_connection?,
            Err(oneshot::error::RecvError { .. }) = &mut serving_stopped => return Ok(()),
        };

        select! {
            biased;
            served_connection = response.serve_connection(stream, &observed_requests) => {
                served_connection?;
            }
            Err(oneshot::error::RecvError { .. }) = &mut serving_stopped => return Ok(()),
        }
    }
}

pub struct LocalHttpFixture {
    pub addr: SocketAddr,
    observed_requests: Arc<ObservedRequests>,
    serving_task: JoinHandle<io::Result<()>>,
    stop_serving: oneshot::Sender<()>,
}

impl LocalHttpFixture {
    /// # Errors
    ///
    /// Returns the [`io::Error`] when the loopback listener cannot be bound or its local address cannot be read.
    pub async fn start(response: FixtureResponse) -> io::Result<Self> {
        TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0)))
            .await
            .and_then(|listener| Self::serving_on(listener, response))
    }

    fn serving_on(listener: TcpListener, response: FixtureResponse) -> io::Result<Self> {
        listener.local_addr().map(|addr| {
            let observed_requests = Arc::new(ObservedRequests::new());
            let (stop_serving, serving_stopped) = oneshot::channel();
            let serving_task = spawn(serve_sequentially_until_stopped(
                listener,
                response,
                observed_requests.clone(),
                serving_stopped,
            ));

            Self {
                addr,
                observed_requests,
                serving_task,
                stop_serving,
            }
        })
    }

    #[must_use]
    pub fn abandoned_trickle_departures(&self) -> Vec<ErrorKind> {
        self.observed_requests.abandoned_trickle_departures()
    }

    #[must_use]
    pub fn last_range_header(&self) -> Option<Vec<u8>> {
        self.observed_requests.last_range_header()
    }

    #[must_use]
    pub fn request_count(&self) -> usize {
        self.observed_requests.count()
    }

    #[must_use]
    pub fn url(&self, path: &str) -> String {
        format!("http://{}{path}", self.addr)
    }

    /// # Errors
    ///
    /// Returns [`RecvError`] when the record of abandoned trickles is dropped before it reaches the expected count.
    pub async fn wait_for_abandoned_trickles(
        &self,
        expected_abandoned_trickle_count: usize,
    ) -> Result<(), RecvError> {
        self.observed_requests
            .wait_for_abandoned_trickles(expected_abandoned_trickle_count)
            .await
    }

    /// # Errors
    ///
    /// Returns [`RecvError`] when the record of requests is dropped before it reaches the expected count.
    pub async fn wait_for_requests(&self, expected_request_count: usize) -> Result<(), RecvError> {
        self.observed_requests
            .wait_for_count(expected_request_count)
            .await
    }
}

impl LocalHttpFixture {
    /// # Errors
    ///
    /// Returns the [`JoinError`] of the serving task when it panicked or was cancelled; the [`io::Error`] that ended serving is the inner result.
    pub async fn stop(self) -> Result<io::Result<()>, JoinError> {
        let Self {
            serving_task,
            stop_serving,
            ..
        } = self;

        drop(stop_serving);

        serving_task.await
    }

    /// # Errors
    ///
    /// Returns the [`JoinError`] of the serving task when it panicked or was cancelled; the [`io::Error`] that ended serving is the inner result.
    pub async fn wait_until_serving_fails(self) -> Result<io::Result<()>, JoinError> {
        let Self {
            serving_task,
            stop_serving,
            ..
        } = self;
        let serving_outcome = serving_task.await;

        drop(stop_serving);

        serving_outcome
    }
}
