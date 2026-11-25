use serde::Deserialize;
use serde::Serialize;

use paddler_inference_parameters::desired_model_runtime_parameters::DesiredModelRuntimeParameters;
use paddler_inference_parameters::embedding_parameters::EmbeddingParameters;

use crate::agent_desired_model::AgentDesiredModel;
use crate::balancer_text_generation_settings::BalancerTextGenerationSettings;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BalancerDesiredState {
    pub embeddings: EmbeddingParameters,
    pub model: AgentDesiredModel,
    pub model_runtime_parameters: DesiredModelRuntimeParameters,
    pub text_generation: BalancerTextGenerationSettings,
}
