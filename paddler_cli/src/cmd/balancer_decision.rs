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
pub struct BalancerDecision {
    #[arg(long, value_parser = parse_socket_addr)]
    /// Address of the TypeSafe-compatible System One API server (enabled only if this address is specified)
    compat_typesafe_addr: Option<ResolvedSocketAddr>,

    #[command(flatten)]
    shared_arguments: BalancerSharedArguments,
}

#[async_trait(?Send)]
impl Handler for BalancerDecision {
    async fn handle(self, shutdown: CancellationToken) -> Result<()> {
        self.shared_arguments
            .run(
                BalancerServingMode::Decision {
                    typesafe_service_configuration: self
                        .compat_typesafe_addr
                        .map(|addr| CompatibilityServiceConfiguration { addr }),
                },
                shutdown,
            )
            .await
    }
}
