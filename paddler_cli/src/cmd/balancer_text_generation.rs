use anyhow::Result;
use async_trait::async_trait;
use clap::Args;
use command_handler::handler::Handler;
use tokio_util::sync::CancellationToken;

use paddler_balancer::compatibility::compatibility_service_configuration::CompatibilityServiceConfiguration;
use paddler_balancer::resolved_socket_addr::ResolvedSocketAddr;
use paddler_balancer_runner::balancer_serving_mode::BalancerServingMode;

use super::balancer_shared_arguments::BalancerSharedArguments;
use super::value_parser::parse_socket_addr::parse_socket_addr;

#[derive(Args)]
pub struct BalancerTextGeneration {
    #[arg(long, value_parser = parse_socket_addr)]
    /// Address of the OpenAI-compatible API server (enabled only if this address is specified)
    compat_openai_addr: Option<ResolvedSocketAddr>,

    #[command(flatten)]
    shared_arguments: BalancerSharedArguments,
}

#[async_trait(?Send)]
impl Handler for BalancerTextGeneration {
    async fn handle(self, shutdown: CancellationToken) -> Result<()> {
        self.shared_arguments
            .run(
                BalancerServingMode::TextGeneration {
                    openai_service_configuration: self
                        .compat_openai_addr
                        .map(|addr| CompatibilityServiceConfiguration { addr }),
                },
                shutdown,
            )
            .await
    }
}
