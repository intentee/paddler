use std::error::Error;
use std::io::Error as IoError;
use std::io::ErrorKind;
use std::net::SocketAddr;
use std::net::TcpListener;

use futures_util::TryFutureExt as _;
use futures_util::future::ready;
use tokio_util::sync::CancellationToken;

use paddler_bootstrap::balancer_runner::BalancerRunner;
use paddler_bootstrap::balancer_runner_params::BalancerRunnerParams;
use paddler_bootstrap::bootstrap_error::BootstrapError;
use paddler_test_cluster_harness::constants::EPHEMERAL_LOOPBACK_ADDR;

use crate::bootstrap_tests_error::BootstrapTestsError;
use crate::ephemeral_balancer_runner_params::ephemeral_balancer_runner_params;

pub struct BalancerRunnerStartOnATakenAddress {
    pub start_error: BootstrapError,
    pub taken_addr: SocketAddr,
}

impl BalancerRunnerStartOnATakenAddress {
    /// # Errors
    ///
    /// Returns [`BootstrapTestsError::OccupyingListenerBindFailed`] when no listener can occupy an ephemeral loopback address, [`BootstrapTestsError::OccupyingListenerAddrUnreadable`] when that listener cannot report the address it occupies, and [`BootstrapTestsError::RunnerStartedOnATakenAddress`] when the balancer runner starts despite the occupied address.
    pub async fn attempt(
        serve_on_the_taken_addr: impl FnOnce(&mut BalancerRunnerParams, SocketAddr),
    ) -> Result<Self, BootstrapTestsError> {
        ready(
            TcpListener::bind(EPHEMERAL_LOOPBACK_ADDR)
                .map_err(BootstrapTestsError::OccupyingListenerBindFailed),
        )
        .and_then(|occupying_listener| {
            Self::attempt_while_occupied(occupying_listener, serve_on_the_taken_addr)
        })
        .await
    }

    async fn attempt_while_occupied(
        occupying_listener: TcpListener,
        serve_on_the_taken_addr: impl FnOnce(&mut BalancerRunnerParams, SocketAddr),
    ) -> Result<Self, BootstrapTestsError> {
        ready(
            occupying_listener
                .local_addr()
                .map_err(BootstrapTestsError::OccupyingListenerAddrUnreadable),
        )
        .and_then(|taken_addr| async move {
            let mut params = ephemeral_balancer_runner_params(CancellationToken::new());

            serve_on_the_taken_addr(&mut params, taken_addr);

            BalancerRunner::start(params)
                .await
                .err()
                .ok_or(BootstrapTestsError::RunnerStartedOnATakenAddress)
                .map(|start_error| Self {
                    start_error,
                    taken_addr,
                })
        })
        .await
    }

    #[must_use]
    pub fn bind_error_kind(&self) -> Option<ErrorKind> {
        self.start_error
            .source()
            .and_then(|source| source.downcast_ref::<IoError>())
            .map(IoError::kind)
    }
}
