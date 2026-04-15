use serde::Deserialize;
use serde::Serialize;

use crate::agent_desired_model::AgentDesiredModel;
use crate::chat_template::ChatTemplate;
use crate::inference_parameters::InferenceParameters;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AgentDesiredState {
    pub chat_template_override: Option<ChatTemplate>,
    pub inference_parameters: InferenceParameters,
    pub model: AgentDesiredModel,
    pub multimodal_projection: AgentDesiredModel,
}
