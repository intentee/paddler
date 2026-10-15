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
use crate::serve_connection::serve_connection;

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
            served_connection = serve_connection(stream, &response, &observed_requests) => {
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
    pub async fn start(response: FixtureResponse) -> io::Result<Self> {
        let listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0))).await?;
        let addr = listener.local_addr()?;
        let observed_requests = Arc::new(ObservedRequests::new());
        let (stop_serving, serving_stopped) = oneshot::channel();
        let serving_task = spawn(serve_sequentially_until_stopped(
            listener,
            response,
            observed_requests.clone(),
            serving_stopped,
        ));

        Ok(Self {
            addr,
            observed_requests,
            serving_task,
            stop_serving,
        })
    }

    #[must_use]
    pub fn url(&self, path: &str) -> String {
        format!("http://{}{path}", self.addr)
    }

    #[must_use]
    pub fn request_count(&self) -> u32 {
        self.observed_requests.count()
    }

    #[must_use]
    pub fn last_range_header(&self) -> Option<Vec<u8>> {
        self.observed_requests.last_range_header()
    }

    pub async fn wait_for_requests(&self, expected_request_count: u32) -> Result<(), RecvError> {
        self.observed_requests
            .wait_for_count(expected_request_count)
            .await
    }

    #[must_use]
    pub fn abandoned_trickle_departures(&self) -> Vec<ErrorKind> {
        self.observed_requests.abandoned_trickle_departures()
    }

    pub async fn wait_for_abandoned_trickles(
        &self,
        expected_abandoned_trickle_count: usize,
    ) -> Result<(), RecvError> {
        self.observed_requests
            .wait_for_abandoned_trickles(expected_abandoned_trickle_count)
            .await
    }
}

impl LocalHttpFixture {
    pub async fn stop(self) -> Result<io::Result<()>, JoinError> {
        let Self {
            serving_task,
            stop_serving,
            ..
        } = self;

        drop(stop_serving);

        serving_task.await
    }

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
