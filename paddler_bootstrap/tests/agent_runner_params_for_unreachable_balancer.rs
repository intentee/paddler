use tokio_util::sync::CancellationToken;
use trzcina::ServiceShutdownOptions;

use paddler_bootstrap::agent_bootstrap_config::AgentBootstrapConfig;
use paddler_bootstrap::agent_runner_params::AgentRunnerParams;

pub fn agent_runner_params_for_unreachable_balancer(
    cancellation_token: CancellationToken,
) -> AgentRunnerParams {
    AgentRunnerParams {
        bootstrap_config: AgentBootstrapConfig {
            agent_name: Some("test-agent".to_owned()),
            management_address: "127.0.0.1:1".to_owned(),
            slots: 1,
        },
        cancellation_token,
        shutdown_options: ServiceShutdownOptions::default(),
    }
}
