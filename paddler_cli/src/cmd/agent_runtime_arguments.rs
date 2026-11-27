use clap::Args;
use clap::value_parser;

use paddler_bootstrap::agent_runtime_config::AgentRuntimeConfig;
use paddler_model_source::desired_model_resolver::DesiredModelResolver;
use paddler_model_source::huggingface_endpoint::HuggingFaceEndpoint;

use crate::cli_error::CliError;
use crate::constants::LLAMA_CPP_MAX_SEQUENCES;

#[derive(Args)]
pub struct AgentRuntimeArguments {
    #[arg(long)]
    /// Hugging Face Hub endpoint that the agent downloads models from (defaults to the `HF_ENDPOINT` environment variable, then to the Hugging Face Hub)
    huggingface_endpoint: Option<String>,

    #[arg(long)]
    /// Name of the agent (optional)
    name: Option<String>,

    #[arg(long, value_parser = value_parser!(u16).range(1..=i64::from(LLAMA_CPP_MAX_SEQUENCES)))]
    /// Number of parallel requests of any kind that the agent can handle at once
    slots: u16,
}

impl AgentRuntimeArguments {
    pub fn runtime_config(self) -> Result<AgentRuntimeConfig, CliError> {
        let Self {
            huggingface_endpoint,
            name,
            slots,
        } = self;

        Ok(AgentRuntimeConfig {
            agent_name: name,
            desired_model_resolver: DesiredModelResolver::from_process_env(
                huggingface_endpoint.map_or(
                    HuggingFaceEndpoint::HubDefault,
                    HuggingFaceEndpoint::Explicit,
                ),
            )
            .map_err(CliError::ModelDownloaderUnconfigurable)?,
            slots,
        })
    }
}
