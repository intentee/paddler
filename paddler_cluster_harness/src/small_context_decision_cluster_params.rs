use paddler_balancer_desired_state_fixture::decision_desired_state::DecisionDesiredState;
use paddler_inference_parameters::batch_size::BatchSize;

use crate::decision_cluster_params::DecisionClusterParams;
use crate::small_decision_context_size::small_decision_context_size;

#[must_use]
pub fn small_context_decision_cluster_params() -> DecisionClusterParams {
    DecisionClusterParams {
        desired_state: DecisionDesiredState {
            context_size: small_decision_context_size(),
            n_batch: BatchSize::default().at_most(small_decision_context_size()),
            ..DecisionDesiredState::default()
        },
        ..DecisionClusterParams::default()
    }
}
