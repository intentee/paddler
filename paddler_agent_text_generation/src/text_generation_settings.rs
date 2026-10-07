use std::num::NonZeroU32;

use paddler_inference_parameters::sampling_parameters::SamplingParameters;
use paddler_messaging::chat_template_source::ChatTemplateSource;

use crate::multimodal_projection::MultimodalProjection;

#[derive(Clone, Debug, PartialEq)]
pub struct TextGenerationSettings {
    pub chat_template_source: ChatTemplateSource,
    pub image_resize_to_fit: NonZeroU32,
    pub multimodal_projection: MultimodalProjection,
    pub sampling_parameters: SamplingParameters,
}
