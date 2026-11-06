use anyhow::Context as _;
use anyhow::Result;
use clap::Args;
use clap::value_parser;

use paddler_bootstrap::agent_runtime_config::AgentRuntimeConfig;
use paddler_bootstrap::llama_cpp_max_sequences::LLAMA_CPP_MAX_SEQUENCES;
use paddler_model_source::desired_model_resolver::DesiredModelResolver;
use paddler_model_source::huggingface_default_endpoint::HUGGINGFACE_DEFAULT_ENDPOINT;

#[derive(Args)]
pub struct AgentRuntimeArguments {
    #[arg(long, env = "HF_ENDPOINT", default_value = HUGGINGFACE_DEFAULT_ENDPOINT)]
    /// Hugging Face Hub endpoint that the agent downloads models from
    huggingface_endpoint: String,

    #[arg(long)]
    /// Name of the agent (optional)
    name: Option<String>,

    #[arg(long, value_parser = value_parser!(u16).range(1..=i64::from(LLAMA_CPP_MAX_SEQUENCES)))]
    /// Number of parallel requests of any kind that the agent can handle at once
    slots: u16,
}

impl AgentRuntimeArguments {
    pub fn runtime_config(self) -> Result<AgentRuntimeConfig> {
        let Self {
            huggingface_endpoint,
            name,
            slots,
        } = self;

        Ok(AgentRuntimeConfig {
            agent_name: name,
            desired_model_resolver: DesiredModelResolver::from_process_env(huggingface_endpoint)
                .context("Unable to set up the model downloader")?,
            slots,
        })
    }
}
