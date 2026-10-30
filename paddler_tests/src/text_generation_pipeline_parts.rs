use tokio_util::sync::CancellationToken;

use paddler_agent_text_generation::multimodal_projection::MultimodalProjection;
use paddler_agent_text_generation::text_generation_pipeline::TextGenerationPipeline;
use paddler_agent_text_generation::text_generation_settings::TextGenerationSettings;
use paddler_inference_parameters::sampling_parameters::SamplingParameters;
use paddler_messaging::chat_template_source::ChatTemplateSource;
use paddler_messaging::multimodal_settings::MultimodalSettings;

use crate::llama_runtime::LlamaRuntime;

pub struct TextGenerationPipelineParts {
    pub chat_template_source: ChatTemplateSource,
    pub llama_runtime: LlamaRuntime,
    pub multimodal_projection: MultimodalProjection,
}

impl TextGenerationPipelineParts {
    #[must_use]
    pub fn into_text_generation_pipeline(self) -> TextGenerationPipeline {
        let Self {
            chat_template_source,
            llama_runtime:
                LlamaRuntime {
                    inference_runtime_context,
                    loaded_llama_model,
                    model_runtime_parameters,
                },
            multimodal_projection,
        } = self;

        TextGenerationPipeline {
            inference_runtime_context,
            loaded_llama_model,
            model_runtime_parameters,
            text_generation_settings: TextGenerationSettings {
                chat_template_source,
                image_resize_to_fit: MultimodalSettings::default().image_resize_to_fit,
                multimodal_projection,
                sampling_parameters: SamplingParameters::default(),
            },
        }
    }

    pub async fn spawn_refusal(self) -> Option<String> {
        self.into_text_generation_pipeline()
            .spawn(&CancellationToken::new())
            .await
            .err()
            .as_ref()
            .map(ToString::to_string)
    }
}
