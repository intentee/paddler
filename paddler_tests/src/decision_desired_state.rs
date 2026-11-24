use std::num::NonZeroU32;

use paddler_cluster_harness::model_card::kev_0_8b::kev_0_8b;
use paddler_inference_parameters::batch_size::BatchSize;
use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;

pub struct DecisionDesiredState {
    pub context_size: NonZeroU32,
    pub n_batch: BatchSize,
}

impl DecisionDesiredState {
    #[must_use]
    pub fn into_balancer_desired_state(self) -> BalancerDesiredState {
        let Self {
            context_size,
            n_batch,
        } = self;
        let desired_state = kev_0_8b().into_desired_state();

        BalancerDesiredState {
            model_runtime_parameters: ModelRuntimeParameters {
                context_size,
                n_batch,
                ..desired_state.model_runtime_parameters
            },
            ..desired_state
        }
    }
}

impl Default for DecisionDesiredState {
    fn default() -> Self {
        let ModelRuntimeParameters {
            context_size,
            n_batch,
            ..
        } = ModelRuntimeParameters::default();

        Self {
            context_size,
            n_batch,
        }
    }
}
