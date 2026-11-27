use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use paddler_agent_model_capabilities::established_model_capabilities::EstablishedModelCapabilities;
use paddler_agent_pipeline_harness::llama_runtime::LlamaRuntime;
use paddler_agent_text_generation::text_generation_error::TextGenerationError;
use paddler_agent_text_generation::text_generation_pipeline::TextGenerationPipeline;
use paddler_agent_text_generation::text_generation_settings::TextGenerationSettings;
use paddler_inference_parameters::desired_sampling_parameters::DesiredSamplingParameters;
use paddler_messaging::multimodal_settings::MultimodalSettings;

pub struct TextGenerationPipelineParts {
    pub llama_runtime: LlamaRuntime,
    pub model_capabilities: Arc<EstablishedModelCapabilities>,
}

impl TextGenerationPipelineParts {
    #[must_use]
    pub fn into_text_generation_pipeline(self) -> TextGenerationPipeline {
        let Self {
            llama_runtime:
                LlamaRuntime {
                    inference_runtime_context,
                    loaded_llama_model,
                    model_runtime_parameters,
                },
            model_capabilities,
        } = self;

        TextGenerationPipeline {
            inference_runtime_context,
            loaded_llama_model,
            model_capabilities,
            model_runtime_parameters,
            text_generation_settings: TextGenerationSettings {
                image_resize_to_fit: MultimodalSettings::default().image_resize_to_fit,
                sampling_parameters: DesiredSamplingParameters::default(),
            },
        }
    }

    pub async fn spawn_refusal(self) -> Option<TextGenerationError> {
        self.into_text_generation_pipeline()
            .spawn(&CancellationToken::new())
            .await
            .err()
    }
}
