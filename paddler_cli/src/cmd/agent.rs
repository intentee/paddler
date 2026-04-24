use anyhow::Result;
use async_trait::async_trait;
use clap::Parser;
use paddler::resolved_socket_addr::ResolvedSocketAddr;
use paddler_bootstrap::agent_runner::AgentRunner;
use paddler_bootstrap::agent_runner::AgentRunnerParams;
use tokio_util::sync::CancellationToken;

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
    async fn handle(&self, shutdown: CancellationToken) -> Result<()> {
        let mut runner = AgentRunner::start(AgentRunnerParams {
            agent_name: self.name.clone(),
            management_address: self.management_addr.socket_addr.to_string(),
            parent_shutdown: Some(shutdown),
            slots: self.slots,
        });

        runner.wait_for_completion().await
    }
}
