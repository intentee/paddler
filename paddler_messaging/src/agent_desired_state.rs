use serde::Deserialize;
use serde::Serialize;

use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;

use crate::agent_desired_model::AgentDesiredModel;
use crate::agent_inference_settings::AgentInferenceSettings;
use crate::balancer_desired_state::BalancerDesiredState;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AgentDesiredState {
    pub inference_settings: AgentInferenceSettings,
    pub model: AgentDesiredModel,
    pub model_runtime_parameters: ModelRuntimeParameters,
}

impl From<BalancerDesiredState> for AgentDesiredState {
    fn from(
        BalancerDesiredState {
            inference_settings,
            model,
            model_runtime_parameters,
        }: BalancerDesiredState,
    ) -> Self {
        Self {
            inference_settings: AgentInferenceSettings::from(inference_settings),
            model,
            model_runtime_parameters,
        }
    }
}
