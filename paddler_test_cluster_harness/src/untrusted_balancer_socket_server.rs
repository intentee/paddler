use std::io::Error as IoError;
use std::net::SocketAddr;

use futures_util::TryFutureExt as _;
use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;

use crate::cluster_harness_error::ClusterHarnessError;
use crate::constants::EPHEMERAL_LOOPBACK_ADDR;
use crate::untrusted_balancer_socket::UntrustedBalancerSocket;

pub struct UntrustedBalancerSocketServer {
    pub management_addr: SocketAddr,
    tcp_listener: TcpListener,
}

impl UntrustedBalancerSocketServer {
    /// # Errors
    ///
    /// Returns the [`IoError`] of a loopback listener that cannot be bound or whose address cannot be read.
    pub async fn bind() -> Result<Self, IoError> {
        TcpListener::bind(EPHEMERAL_LOOPBACK_ADDR)
            .await
            .and_then(|tcp_listener| {
                tcp_listener.local_addr().map(|management_addr| Self {
                    management_addr,
                    tcp_listener,
                })
            })
    }

    /// # Errors
    ///
    /// Returns [`ClusterHarnessError::BalancerSocketAcceptFailed`] when no agent connection can be accepted, and [`ClusterHarnessError::BalancerSocketHandshakeFailed`] when the agent's WebSocket handshake fails.
    pub async fn accept_agent(&self) -> Result<UntrustedBalancerSocket, ClusterHarnessError> {
        self.tcp_listener
            .accept()
            .map_err(ClusterHarnessError::BalancerSocketAcceptFailed)
            .and_then(|(tcp_stream, _agent_addr)| {
                accept_async(tcp_stream)
                    .map_ok(UntrustedBalancerSocket::new)
                    .map_err(ClusterHarnessError::BalancerSocketHandshakeFailed)
            })
            .await
    }
}
