use std::io::Error as IoError;
use std::net::SocketAddr;

use futures_util::TryFutureExt as _;
use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;

use crate::cluster_harness_error::ClusterHarnessError;
use crate::ephemeral_loopback_addr::EPHEMERAL_LOOPBACK_ADDR;
use crate::untrusted_balancer_socket::UntrustedBalancerSocket;

pub struct UntrustedBalancerSocketServer {
    pub management_addr: SocketAddr,
    tcp_listener: TcpListener,
}

impl UntrustedBalancerSocketServer {
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
