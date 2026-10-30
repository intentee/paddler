use std::net::SocketAddr;

use tokio_util::sync::CancellationToken;

use paddler_bootstrap::agent_bootstrap_config::AgentBootstrapConfig;
use paddler_bootstrap::agent_runner::AgentRunner;
use paddler_bootstrap::agent_runner_params::AgentRunnerParams;
use paddler_model_source::desired_model_resolver::DesiredModelResolver;
use paddler_model_source::huggingface_default_endpoint::HUGGINGFACE_DEFAULT_ENDPOINT;
use paddler_test_cluster_harness::unexpiring_shutdown_options::unexpiring_shutdown_options;

#[must_use]
pub fn agent_runner_connected_to(management_addr: SocketAddr, slots: u16) -> AgentRunner {
    AgentRunner::start(AgentRunnerParams {
        bootstrap_config: AgentBootstrapConfig {
            agent_name: Some("test-agent".to_owned()),
            desired_model_resolver: DesiredModelResolver::from_process_env(
                HUGGINGFACE_DEFAULT_ENDPOINT.to_owned(),
            )
            .expect("the model downloader must start"),
            management_address: management_addr.to_string(),
            slots,
        },
        cancellation_token: CancellationToken::new(),
        shutdown_options: unexpiring_shutdown_options(),
    })
}
