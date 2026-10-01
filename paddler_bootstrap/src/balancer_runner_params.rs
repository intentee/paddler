use tokio_util::sync::CancellationToken;
use trzcina::ServiceShutdownOptions;

use crate::balancer_bootstrap_config::BalancerBootstrapConfig;

pub struct BalancerRunnerParams {
    pub bootstrap_config: BalancerBootstrapConfig,
    pub cancellation_token: CancellationToken,
    pub shutdown_options: ServiceShutdownOptions,
}
