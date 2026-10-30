#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use paddler_inference_parameters::batch_size::BatchSize;
use paddler_tests::decision_cluster_params::DecisionClusterParams;
use paddler_tests::decision_desired_state::DecisionDesiredState;

pub const SMALL_DECISION_CONTEXT_SIZE: u32 = 256;

pub fn small_context_decision_cluster_params() -> DecisionClusterParams {
    DecisionClusterParams {
        desired_state: DecisionDesiredState {
            context_size: NonZeroU32::new(SMALL_DECISION_CONTEXT_SIZE)
                .expect("the context size is non-zero"),
            n_batch: BatchSize::try_from(SMALL_DECISION_CONTEXT_SIZE)
                .expect("the batch fits the context"),
            ..DecisionDesiredState::default()
        },
        ..DecisionClusterParams::default()
    }
}
