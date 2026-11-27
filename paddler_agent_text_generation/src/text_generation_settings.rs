use std::num::NonZeroU32;

use paddler_inference_parameters::desired_sampling_parameters::DesiredSamplingParameters;

#[derive(Clone, Debug, PartialEq)]
pub struct TextGenerationSettings {
    pub image_resize_to_fit: NonZeroU32,
    pub sampling_parameters: DesiredSamplingParameters,
}
