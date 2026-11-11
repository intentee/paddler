use paddler_inference_parameters::batch_size::BatchSize;

use crate::constants::SMALL_DECISION_CONTEXT_SIZE;
use crate::decision_cluster_params::DecisionClusterParams;
use crate::decision_desired_state::DecisionDesiredState;

#[must_use]
pub fn small_context_decision_cluster_params() -> DecisionClusterParams {
    DecisionClusterParams {
        desired_state: DecisionDesiredState {
            context_size: SMALL_DECISION_CONTEXT_SIZE,
            n_batch: BatchSize::default().at_most(SMALL_DECISION_CONTEXT_SIZE),
            ..DecisionDesiredState::default()
        },
        ..DecisionClusterParams::default()
    }
}
