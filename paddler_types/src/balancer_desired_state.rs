use serde::Deserialize;
use serde::Serialize;

use crate::agent_desired_model::AgentDesiredModel;
use crate::chat_template::ChatTemplate;
use crate::inference_parameters::InferenceParameters;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BalancerDesiredState {
    pub chat_template_override: Option<ChatTemplate>,
    pub inference_parameters: InferenceParameters,
    pub model: AgentDesiredModel,
    pub use_chat_template_override: bool,
}
