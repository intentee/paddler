use paddler_agent_model_capabilities::model_capabilities_settings::ModelCapabilitiesSettings;
use paddler_agent_model_capabilities::multimodal_projection::MultimodalProjection;
use paddler_inference_parameters::pooling_type::PoolingType;
use paddler_messaging::chat_template_source::ChatTemplateSource;

#[must_use]
pub const fn default_model_capabilities_settings() -> ModelCapabilitiesSettings {
    ModelCapabilitiesSettings {
        chat_template_source: ChatTemplateSource::EmbeddedInModel,
        embedding_pooling: PoolingType::Unspecified,
        multimodal_projection: MultimodalProjection::NotConfigured,
    }
}
