use anyhow::Result;
use async_trait::async_trait;
use clap::Parser;
use command_handler::handler::Handler;
use tokio_util::sync::CancellationToken;

use paddler_bootstrap::singleton_bootstrap_config::SingletonBootstrapConfig;
use paddler_bootstrap::singleton_service_bundle::SingletonServiceBundle;

use super::agent_runtime_arguments::AgentRuntimeArguments;
use super::balancer::Balancer;
use super::run_announced_service_manager::run_announced_service_manager;

#[derive(Parser)]
pub struct Singleton {
    #[command(flatten)]
    agent_runtime: AgentRuntimeArguments,

    #[command(flatten)]
    balancer: Balancer,
}

impl Singleton {
    async fn run_singleton(self, shutdown: CancellationToken) -> Result<()> {
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

#[async_trait(?Send)]
impl Handler for Singleton {
    async fn handle(self, shutdown: CancellationToken) -> Result<()> {
        self.run_singleton(shutdown).await
    }
}
