use std::io;
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::TcpListener;
use tokio::spawn;
use tokio::task::JoinHandle;

use crate::fixture_response::FixtureResponse;
use crate::observed_requests::ObservedRequests;
use crate::serve_connection::serve_connection;

async fn serve_sequentially(
    listener: TcpListener,
    response: FixtureResponse,
    observed_requests: Arc<ObservedRequests>,
) -> io::Result<()> {
    loop {
        let (stream, _peer_addr) = listener.accept().await?;

        serve_connection(stream, &response, &observed_requests).await?;
    }
}

pub struct LocalHttpFixture {
    pub addr: SocketAddr,
    observed_requests: Arc<ObservedRequests>,
    serving_task: JoinHandle<io::Result<()>>,
}

impl LocalHttpFixture {
    pub async fn start(response: FixtureResponse) -> io::Result<Self> {
        let listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0))).await?;
        let addr = listener.local_addr()?;
        let observed_requests = Arc::new(ObservedRequests::new());
        let serving_task = spawn(serve_sequentially(
            listener,
            response,
            observed_requests.clone(),
        ));

        Ok(Self {
            addr,
            observed_requests,
            serving_task,
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
}

impl Drop for LocalHttpFixture {
    fn drop(&mut self) {
        self.serving_task.abort();
    }
}
