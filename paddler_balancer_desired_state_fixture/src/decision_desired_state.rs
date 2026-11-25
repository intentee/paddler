use std::num::NonZeroU32;

use paddler_inference_parameters::batch_size::BatchSize;
use paddler_inference_parameters::desired_model_runtime_parameters::DesiredModelRuntimeParameters;
use paddler_inference_parameters::model_default_or::ModelDefaultOr;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_model_card_fixture::constants::SERVED_CONTEXT_SIZE;
use paddler_model_card_fixture::kev_0_8b::kev_0_8b;
use paddler_model_card_fixture::model_card::ModelCard;

pub struct DecisionDesiredState {
    pub context_size: NonZeroU32,
    pub model_card: ModelCard,
    pub n_batch: BatchSize,
}

impl DecisionDesiredState {
    #[must_use]
    pub fn into_balancer_desired_state(self) -> BalancerDesiredState {
        let Self {
            context_size,
            model_card,
            n_batch,
        } = self;
        let desired_state = model_card.into_desired_state();

        BalancerDesiredState {
            model_runtime_parameters: DesiredModelRuntimeParameters {
                context_size: ModelDefaultOr::Explicit(context_size),
                n_batch: ModelDefaultOr::Explicit(n_batch),
                ..desired_state.model_runtime_parameters
            },
            ..desired_state
        }
    }
}

impl Default for DecisionDesiredState {
    fn default() -> Self {
        Self {
            context_size: SERVED_CONTEXT_SIZE,
            model_card: kev_0_8b(),
            n_batch: BatchSize::default(),
        }
    }
}
