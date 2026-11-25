use std::net::Ipv4Addr;
use std::net::SocketAddr;

use tokio_util::sync::CancellationToken;

use paddler_bootstrap::agent_runner_params::AgentRunnerParams;
use paddler_model_source::model_source_error::ModelSourceError;

use crate::agent_runner_params_for_balancer_at::agent_runner_params_for_balancer_at;

/// # Errors
///
/// Returns the [`ModelSourceError`] of [`agent_runner_params_for_balancer_at`] when the model downloader cannot start.
pub fn agent_runner_params_for_unreachable_balancer(
    cancellation_token: CancellationToken,
) -> Result<AgentRunnerParams, ModelSourceError> {
    agent_runner_params_for_balancer_at(
        cancellation_token,
        SocketAddr::from((Ipv4Addr::LOCALHOST, 1)),
    )
}
