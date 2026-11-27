use paddler_inference_parameters::pooling_type::PoolingType;
use paddler_messaging::chat_template_source::ChatTemplateSource;

use crate::multimodal_projection::MultimodalProjection;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelCapabilitiesSettings {
    pub chat_template_source: ChatTemplateSource,
    pub embedding_pooling: PoolingType,
    pub multimodal_projection: MultimodalProjection,
}
