use serde::Deserialize;
use serde::Serialize;

use paddler_inference_parameters::embedding_parameters::EmbeddingParameters;
use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;

use crate::agent_desired_model::AgentDesiredModel;
use crate::balancer_text_generation_settings::BalancerTextGenerationSettings;
use crate::decision_settings::DecisionSettings;
use crate::inference_mode::InferenceMode;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BalancerDesiredState {
    pub decision: DecisionSettings,
    pub embeddings: EmbeddingParameters,
    pub inference_mode: InferenceMode,
    pub model: AgentDesiredModel,
    pub model_runtime_parameters: ModelRuntimeParameters,
    pub text_generation: BalancerTextGenerationSettings,
}
