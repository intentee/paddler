use std::future::pending;
use std::io;
use std::net::SocketAddr;

use futures_util::StreamExt as _;
use futures_util::stream::FuturesUnordered;
use tokio::io::copy_bidirectional;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::select;
use tokio::spawn;
use tokio::sync::watch;
use tokio::sync::watch::error::RecvError;
use tokio::task::JoinError;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

async fn relay_until_stalled(
    mut downstream: TcpStream,
    upstream_addr: SocketAddr,
    stall: CancellationToken,
) -> io::Result<()> {
    let mut upstream = TcpStream::connect(upstream_addr).await?;

    select! {
        relay_outcome = copy_bidirectional(&mut downstream, &mut upstream) => relay_outcome.map(drop),
        () = stall.cancelled() => pending().await,
    }
}

async fn relay_accepted_connections(
    listener: TcpListener,
    upstream_addr: SocketAddr,
    accepted_connection_count_tx: watch::Sender<u32>,
    stall_rx: watch::Receiver<CancellationToken>,
    stop: CancellationToken,
) -> io::Result<()> {
    let mut relayed_connections = FuturesUnordered::new();

    loop {
        select! {
            () = stop.cancelled() => break Ok(()),
            accepted_connection = listener.accept() => {
                let (downstream, _downstream_addr) = accepted_connection?;

                accepted_connection_count_tx.send_modify(|accepted_connection_count| {
                    *accepted_connection_count += 1;
                });
                relayed_connections.push(relay_until_stalled(
                    downstream,
                    upstream_addr,
                    stall_rx.borrow().clone(),
                ));
            }
            Some(relayed_connection) = relayed_connections.next() => relayed_connection?,
        }
    }
}

pub struct StallableTcpRelay {
    accepted_connection_count_rx: watch::Receiver<u32>,
    relaying_task: JoinHandle<io::Result<()>>,
    stall_tx: watch::Sender<CancellationToken>,
    stop: CancellationToken,
}

impl StallableTcpRelay {
    #[must_use]
    pub fn start(listener: TcpListener, upstream_addr: SocketAddr) -> Self {
        let (accepted_connection_count_tx, accepted_connection_count_rx) = watch::channel(0);
        let (stall_tx, stall_rx) = watch::channel(CancellationToken::new());
        let stop = CancellationToken::new();
        let relaying_task = spawn(relay_accepted_connections(
            listener,
            upstream_addr,
            accepted_connection_count_tx,
            stall_rx,
            stop.clone(),
        ));

        Self {
            accepted_connection_count_rx,
            relaying_task,
            stall_tx,
            stop,
        }
    }

    pub fn stall_established_connections(&self) {
        self.stall_tx
            .send_replace(CancellationToken::new())
            .cancel();
    }

    pub async fn wait_for_accepted_connections(
        &self,
        expected_accepted_connection_count: u32,
    ) -> Result<(), RecvError> {
        self.accepted_connection_count_rx
            .clone()
            .wait_for(|accepted_connection_count| {
                *accepted_connection_count >= expected_accepted_connection_count
            })
            .await
            .map(drop)
    }

    pub async fn wait_until_relaying_ends(self) -> Result<io::Result<()>, JoinError> {
        self.relaying_task.await
    }

    pub async fn stop(self) -> Result<io::Result<()>, JoinError> {
        self.stop.cancel();
        self.relaying_task.await
    }
}
