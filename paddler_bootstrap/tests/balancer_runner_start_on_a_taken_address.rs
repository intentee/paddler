use std::error::Error;
use std::io::Error as IoError;
use std::io::ErrorKind;
use std::net::SocketAddr;
use std::net::TcpListener;

use tokio_util::sync::CancellationToken;

use paddler_bootstrap::balancer_runner::BalancerRunner;
use paddler_bootstrap::balancer_runner_params::BalancerRunnerParams;
use paddler_bootstrap::bootstrap_error::BootstrapError;
use paddler_test_cluster_harness::ephemeral_loopback_addr::EPHEMERAL_LOOPBACK_ADDR;

use crate::ephemeral_balancer_runner_params::ephemeral_balancer_runner_params;

pub struct BalancerRunnerStartOnATakenAddress {
    pub start_error: BootstrapError,
    pub taken_addr: SocketAddr,
}

impl BalancerRunnerStartOnATakenAddress {
    pub async fn attempt(
        serve_on_the_taken_addr: impl FnOnce(&mut BalancerRunnerParams, SocketAddr),
    ) -> Self {
        let occupying_listener = TcpListener::bind(EPHEMERAL_LOOPBACK_ADDR)
            .expect("an ephemeral loopback port must bind");
        let taken_addr = occupying_listener
            .local_addr()
            .expect("a bound listener must report its address");
        let mut params = ephemeral_balancer_runner_params(CancellationToken::new());

        serve_on_the_taken_addr(&mut params, taken_addr);

        Self {
            start_error: BalancerRunner::start(params)
                .await
                .err()
                .expect("a runner must not start on a taken address"),
            taken_addr,
        }
    }

    pub fn bind_error_kind(&self) -> Option<ErrorKind> {
        self.start_error
            .source()
            .and_then(|source| source.downcast_ref::<IoError>())
            .map(IoError::kind)
    }
}
