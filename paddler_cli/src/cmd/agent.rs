use anyhow::Result;
use async_trait::async_trait;
use clap::Parser;
use clap::value_parser;
use command_handler::handler::Handler;
use tokio_util::sync::CancellationToken;
use trzcina::ServiceShutdownOptions;

use paddler_balancer::resolved_socket_addr::ResolvedSocketAddr;
use paddler_bootstrap::agent_bootstrap_config::AgentBootstrapConfig;
use paddler_bootstrap::agent_service_bundle::AgentServiceBundle;
use paddler_bootstrap::llama_cpp_max_sequences::LLAMA_CPP_MAX_SEQUENCES;
use paddler_bootstrap::run_service_manager::run_service_manager;

use super::value_parser::parse_socket_addr::parse_socket_addr;

#[derive(Parser)]
pub struct Agent {
    #[arg(long, value_parser = parse_socket_addr)]
    /// Address of the management server that the agent will connect to
    management_addr: ResolvedSocketAddr,

    #[arg(long)]
    /// Name of the agent (optional)
    name: Option<String>,

    #[arg(long, value_parser = value_parser!(u16).range(1..=i64::from(LLAMA_CPP_MAX_SEQUENCES)))]
    /// Number of parallel requests of any kind that the agent can handle at once
    slots: u16,
}

#[async_trait(?Send)]
impl Handler for Agent {
    async fn handle(self, shutdown: CancellationToken) -> Result<()> {
        let Self {
            management_addr,
            name,
            slots,
        } = self;

        run_service_manager(
            AgentServiceBundle::new(AgentBootstrapConfig {
                agent_name: name,
                management_address: management_addr.socket_addr.to_string(),
                slots,
            }),
            shutdown,
            ServiceShutdownOptions::default(),
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser as _;
    use clap::error::ErrorKind;

    use super::Agent;

    #[test]
    fn rejects_an_agent_without_slots() {
        let parse_error = Agent::try_parse_from([
            "agent",
            "--management-addr",
            "127.0.0.1:8060",
            "--slots",
            "0",
        ])
        .err()
        .expect("an agent with zero slots must be rejected");

        assert_eq!(parse_error.kind(), ErrorKind::ValueValidation);
    }
}
