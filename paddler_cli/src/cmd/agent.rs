use clap::Parser;
use tokio_util::sync::CancellationToken;
use trzcina::service_shutdown_options::ServiceShutdownOptions;

use paddler_balancer::resolved_socket_addr::ResolvedSocketAddr;
use paddler_bootstrap::agent_bootstrap_config::AgentBootstrapConfig;
use paddler_bootstrap::agent_service_bundle::AgentServiceBundle;
use paddler_bootstrap::run_service_manager::run_service_manager;

use super::agent_runtime_arguments::AgentRuntimeArguments;
use super::value_parser::parse_socket_addr::parse_socket_addr;
use crate::cli_error::CliError;

#[derive(Parser)]
pub struct Agent {
    #[arg(long, value_parser = parse_socket_addr)]
    /// Address of the management server that the agent will connect to
    management_addr: ResolvedSocketAddr,

    #[command(flatten)]
    runtime: AgentRuntimeArguments,
}

impl Agent {
    pub async fn run(self, shutdown: CancellationToken) -> Result<(), CliError> {
        let Self {
            management_addr,
            runtime,
        } = self;

        run_service_manager(
            AgentServiceBundle::new(AgentBootstrapConfig {
                management_address: management_addr.socket_addr,
                runtime: runtime.runtime_config()?,
            }),
            shutdown,
            ServiceShutdownOptions::default(),
        )
        .await
        .map_err(CliError::from)
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
