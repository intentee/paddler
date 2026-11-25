use std::net::SocketAddr;

use tokio_util::sync::CancellationToken;

use paddler_bootstrap::agent_bootstrap_config::AgentBootstrapConfig;
use paddler_bootstrap::agent_runner::AgentRunner;
use paddler_bootstrap::agent_runner_params::AgentRunnerParams;
use paddler_bootstrap::agent_runtime_config::AgentRuntimeConfig;
use paddler_cluster_harness::unexpiring_shutdown_options::unexpiring_shutdown_options;
use paddler_model_source::desired_model_resolver::DesiredModelResolver;
use paddler_model_source::huggingface_endpoint::HuggingFaceEndpoint;
use paddler_model_source::model_source_error::ModelSourceError;

/// # Errors
///
/// Returns the [`ModelSourceError`] of [`DesiredModelResolver::from_process_env`] when the model downloader cannot start.
pub fn agent_runner_connected_to(
    management_addr: SocketAddr,
    slots: u16,
) -> Result<AgentRunner, ModelSourceError> {
    DesiredModelResolver::from_process_env(HuggingFaceEndpoint::HubDefault).map(
        |desired_model_resolver| {
            AgentRunner::start(AgentRunnerParams {
                bootstrap_config: AgentBootstrapConfig {
                    management_address: management_addr,
                    runtime: AgentRuntimeConfig {
                        agent_name: Some("test-agent".to_owned()),
                        desired_model_resolver,
                        slots,
                    },
                },
                cancellation_token: CancellationToken::new(),
                shutdown_options: unexpiring_shutdown_options(),
            })
        },
    )
}
