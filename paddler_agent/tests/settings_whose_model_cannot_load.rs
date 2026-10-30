use std::num::NonZeroU32;
use std::path::PathBuf;

use paddler_agent::applicable_decisions::ApplicableDecisions;
use paddler_agent::inference_capabilities::InferenceCapabilities;
use paddler_agent::model_serving_settings::ModelServingSettings;
use paddler_agent_text_generation::multimodal_projection::MultimodalProjection;
use paddler_agent_text_generation::text_generation_settings::TextGenerationSettings;
use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
use paddler_inference_parameters::pooling_type::PoolingType;
use paddler_inference_parameters::sampling_parameters::SamplingParameters;
use paddler_messaging::chat_template_source::ChatTemplateSource;

pub fn settings_whose_model_cannot_load() -> ModelServingSettings {
    ModelServingSettings {
        capabilities: InferenceCapabilities {
            decisions: ApplicableDecisions::Disabled,
            embeddings: PoolingType::Last,
            text_generation: TextGenerationSettings {
                chat_template_source: ChatTemplateSource::EmbeddedInModel,
                image_resize_to_fit: NonZeroU32::MIN,
                multimodal_projection: MultimodalProjection::NotConfigured,
                sampling_parameters: SamplingParameters::default(),
            },
        },
        model_path: PathBuf::from("/nonexistent/model.gguf"),
        model_runtime_parameters: ModelRuntimeParameters::default(),
    }
}
