use paddler_bootstrap::agent_runner_params::AgentRunnerParams;
use tokio_util::sync::CancellationToken;

pub fn agent_runner_params_for_unreachable_balancer(
    cancellation_token: CancellationToken,
) -> AgentRunnerParams {
    AgentRunnerParams {
        agent_name: Some("test-agent".to_owned()),
        cancellation_token,
        management_address: "127.0.0.1:1".to_owned(),
        slots: 1,
    }
}
