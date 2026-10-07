use tokio_util::sync::CancellationToken;
use trzcina::ServiceShutdownOptions;

use crate::agent_runner_config::AgentRunnerConfig;

pub struct AgentRunnerParams {
    pub runner_config: AgentRunnerConfig,
    pub cancellation_token: CancellationToken,
    pub shutdown_options: ServiceShutdownOptions,
}
