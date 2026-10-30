use tokio_util::sync::CancellationToken;

use paddler_bootstrap::agent_runner_params::AgentRunnerParams;

use crate::agent_runner_params_for_balancer_at::agent_runner_params_for_balancer_at;

pub fn agent_runner_params_for_unreachable_balancer(
    cancellation_token: CancellationToken,
) -> AgentRunnerParams {
    agent_runner_params_for_balancer_at(cancellation_token, "127.0.0.1:1".to_owned())
}
