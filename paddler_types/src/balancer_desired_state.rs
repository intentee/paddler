use serde::Deserialize;
use serde::Serialize;

use crate::agent_desired_model::AgentDesiredModel;
use crate::agent_desired_state::AgentDesiredState;
use crate::chat_template::ChatTemplate;
use crate::inference_parameters::InferenceParameters;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BalancerDesiredState {
    pub chat_template_override: Option<ChatTemplate>,
    pub inference_parameters: InferenceParameters,
    pub model: AgentDesiredModel,
    pub multimodal_projection: AgentDesiredModel,
    pub use_chat_template_override: bool,
}

impl BalancerDesiredState {
    #[must_use]
    pub fn to_agent_desired_state(&self) -> AgentDesiredState {
        AgentDesiredState {
            chat_template_override: if self.use_chat_template_override {
                self.chat_template_override.clone()
            } else {
                None
            },
            inference_parameters: self.inference_parameters.clone(),
            model: self.model.clone(),
            multimodal_projection: self.multimodal_projection.clone(),
        }
    }
}
