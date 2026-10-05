use paddler_inference_parameters::inference_parameters::InferenceParameters;
use paddler_messaging::chat_template::ChatTemplate;

use crate::agent_applicable_model::AgentApplicableModel;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AgentApplicableState {
    pub chat_template_override: Option<ChatTemplate>,
    pub inference_parameters: InferenceParameters,
    pub model: AgentApplicableModel,
}
