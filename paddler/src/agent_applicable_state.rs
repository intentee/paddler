use std::path::PathBuf;

use paddler_types::chat_template::ChatTemplate;
use paddler_types::inference_parameters::InferenceParameters;

#[derive(Clone, Debug)]
pub struct AgentApplicableState {
    pub chat_template_override: Option<ChatTemplate>,
    pub inference_parameters: InferenceParameters,
    pub multimodal_projection_path: Option<PathBuf>,
    pub model_path: Option<PathBuf>,
}
