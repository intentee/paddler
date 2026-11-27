use std::num::NonZeroU32;
use std::path::PathBuf;

use paddler_agent::model_serving_settings::ModelServingSettings;
use paddler_agent_model_capabilities::model_capabilities_settings::ModelCapabilitiesSettings;
use paddler_agent_model_capabilities::multimodal_projection::MultimodalProjection;
use paddler_agent_text_generation::text_generation_settings::TextGenerationSettings;
use paddler_inference_parameters::desired_model_runtime_parameters::DesiredModelRuntimeParameters;
use paddler_inference_parameters::desired_sampling_parameters::DesiredSamplingParameters;
use paddler_inference_parameters::pooling_type::PoolingType;
use paddler_messaging::chat_template_source::ChatTemplateSource;

#[must_use]
pub fn settings_whose_model_cannot_load() -> ModelServingSettings {
    ModelServingSettings {
        model_capabilities: ModelCapabilitiesSettings {
            chat_template_source: ChatTemplateSource::EmbeddedInModel,
            embedding_pooling: PoolingType::Last,
            multimodal_projection: MultimodalProjection::NotConfigured,
        },
        model_path: PathBuf::from("/nonexistent/model.gguf"),
        model_runtime_parameters: DesiredModelRuntimeParameters::default(),
        text_generation: TextGenerationSettings {
            image_resize_to_fit: NonZeroU32::MIN,
            sampling_parameters: DesiredSamplingParameters::default(),
        },
    }
}
