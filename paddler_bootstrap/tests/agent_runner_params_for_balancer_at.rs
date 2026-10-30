use tokio_util::sync::CancellationToken;
use trzcina::ServiceShutdownOptions;

use paddler_bootstrap::agent_bootstrap_config::AgentBootstrapConfig;
use paddler_bootstrap::agent_runner_params::AgentRunnerParams;
use paddler_model_source::desired_model_resolver::DesiredModelResolver;
use paddler_model_source::huggingface_default_endpoint::HUGGINGFACE_DEFAULT_ENDPOINT;

pub fn agent_runner_params_for_balancer_at(
    cancellation_token: CancellationToken,
    management_address: String,
) -> AgentRunnerParams {
    AgentRunnerParams {
        bootstrap_config: AgentBootstrapConfig {
            agent_name: Some("test-agent".to_owned()),
            desired_model_resolver: DesiredModelResolver::from_process_env(
                HUGGINGFACE_DEFAULT_ENDPOINT.to_owned(),
            )
            .expect("the model downloader must start"),
            management_address,
            slots: 1,
        },
        cancellation_token,
        shutdown_options: ServiceShutdownOptions::default(),
    }
}
