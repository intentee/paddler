use anyhow::Result;
use async_trait::async_trait;
use clap::Parser;
use command_handler::handler::Handler;
use paddler_balancer::resolved_socket_addr::ResolvedSocketAddr;
use paddler_bootstrap::agent_service_bundle::AgentServiceBundle;
use paddler_bootstrap::run_service_manager::run_service_manager;
use tokio_util::sync::CancellationToken;
use trzcina::ServiceShutdownOptions;

use super::value_parser::parse_socket_addr::parse_socket_addr;

#[derive(Parser)]
pub struct Agent {
    #[arg(long, value_parser = parse_socket_addr)]
    /// Address of the management server that the agent will connect to
    management_addr: ResolvedSocketAddr,

    #[arg(long)]
    /// Name of the agent (optional)
    name: Option<String>,

    #[arg(long)]
    /// Number of parallel requests of any kind that the agent can handle at once
    slots: i32,
}

#[async_trait(?Send)]
impl Handler for Agent {
    async fn handle(self, shutdown: CancellationToken) -> Result<()> {
        run_service_manager(
            AgentServiceBundle::new(
                self.name.clone(),
                &self.management_addr.socket_addr.to_string(),
                self.slots,
            ),
            shutdown,
            ServiceShutdownOptions::default(),
        )
        .await
    }
}
