use anyhow::Result;
use async_trait::async_trait;
use clap::Parser;
use paddler::resolved_socket_addr::ResolvedSocketAddr;
use paddler_bootstrap::agent::Agent as BootstrappedAgent;
use paddler_bootstrap::agent_params::AgentParams;
use tokio::sync::oneshot;

use super::handler::Handler;
use super::value_parser::parse_socket_addr;

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

#[async_trait]
impl Handler for Agent {
    async fn handle(&self, shutdown_rx: oneshot::Receiver<()>) -> Result<()> {
        let bootstrapped = BootstrappedAgent::bootstrap(AgentParams {
            agent_name: self.name.clone(),
            management_address: self.management_addr.socket_addr.to_string(),
            slots: self.slots,
        });

        bootstrapped.service_manager.run_forever(shutdown_rx).await
    }
}
