use tokio_util::sync::CancellationToken;
use trzcina::ServiceShutdownOptions;

use crate::balancer_runner_config::BalancerRunnerConfig;

pub struct BalancerRunnerParams {
    pub runner_config: BalancerRunnerConfig,
    pub cancellation_token: CancellationToken,
    pub shutdown_options: ServiceShutdownOptions,
}
