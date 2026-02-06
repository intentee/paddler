use std::path::PathBuf;

use crate::chat_template::ChatTemplate;
use crate::inference_parameters::InferenceParameters;

#[derive(Clone, Debug)]
pub struct AgentApplicableState {
    pub chat_template_override: Option<ChatTemplate>,
    pub inference_parameters: InferenceParameters,
    pub model_path: Option<PathBuf>,
}
