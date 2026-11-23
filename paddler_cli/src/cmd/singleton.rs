use clap::Parser;
use tokio_util::sync::CancellationToken;

use paddler_bootstrap::singleton_bootstrap_config::SingletonBootstrapConfig;
use paddler_bootstrap::singleton_service_bundle::SingletonServiceBundle;

use super::agent_runtime_arguments::AgentRuntimeArguments;
use super::balancer::Balancer;
use super::run_announced_service_manager::run_announced_service_manager;
use crate::cli_error::CliError;

#[derive(Parser)]
pub struct Singleton {
    #[command(flatten)]
    agent_runtime: AgentRuntimeArguments,

    #[command(flatten)]
    balancer: Balancer,
}

impl Singleton {
    pub async fn run(self, shutdown: CancellationToken) -> Result<(), CliError> {
        let Self {
            agent_runtime,
            balancer,
        } = self;

        let bundle = SingletonServiceBundle::new(SingletonBootstrapConfig {
            agent_runtime: agent_runtime.runtime_config()?,
            balancer: balancer.bootstrap_config(),
        })
        .await?;

        run_announced_service_manager(bundle.balancer.addresses, bundle, shutdown).await
    }
}
