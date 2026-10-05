use tokio_util::sync::CancellationToken;
use trzcina::ServiceShutdownOptions;

use crate::agent_bootstrap_config::AgentBootstrapConfig;

pub struct AgentRunnerParams {
    pub bootstrap_config: AgentBootstrapConfig,
    pub cancellation_token: CancellationToken,
    pub shutdown_options: ServiceShutdownOptions,
}
