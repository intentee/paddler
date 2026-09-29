use std::path::PathBuf;

use paddler_inference_parameters::inference_parameters::InferenceParameters;
use paddler_messaging::chat_template::ChatTemplate;

#[derive(Clone, Debug)]
pub struct AgentApplicableState {
    pub chat_template_override: Option<ChatTemplate>,
    pub inference_parameters: InferenceParameters,
    pub multimodal_projection_path: Option<PathBuf>,
    pub model_path: Option<PathBuf>,
}
