use serde::Deserialize;
use serde::Serialize;

use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;

use crate::agent_desired_model::AgentDesiredModel;
use crate::balancer_inference_settings::BalancerInferenceSettings;
use crate::inference_mode::InferenceMode;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BalancerDesiredState {
    pub inference_settings: BalancerInferenceSettings,
    pub model: AgentDesiredModel,
    pub model_runtime_parameters: ModelRuntimeParameters,
}

impl BalancerDesiredState {
    #[must_use]
    pub fn unconfigured(inference_mode: InferenceMode) -> Self {
        Self {
            inference_settings: BalancerInferenceSettings::unconfigured(inference_mode),
            model: AgentDesiredModel::None,
            model_runtime_parameters: ModelRuntimeParameters::default(),
        }
    }
}
