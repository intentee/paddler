use serde::Deserialize;
use serde::Serialize;

use paddler_inference_parameters::sampling_parameters::SamplingParameters;

use crate::chat_template_source::ChatTemplateSource;
use crate::multimodal_settings::MultimodalSettings;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AgentTextGenerationSettings {
    pub chat_template_source: ChatTemplateSource,
    pub multimodal: MultimodalSettings,
    pub sampling_parameters: SamplingParameters,
}
