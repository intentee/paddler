use paddler_inference_parameters::inference_parameters::InferenceParameters;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;

use crate::qwen3_desired_state::qwen3_desired_state;

#[must_use]
pub fn qwen3_desired_state_with_embeddings(enable_embeddings: bool) -> BalancerDesiredState {
    let desired_state = qwen3_desired_state();

    BalancerDesiredState {
        inference_parameters: InferenceParameters {
            enable_embeddings,
            ..desired_state.inference_parameters
        },
        ..desired_state
    }
}
