#![cfg(feature = "tests_that_use_llms")]

use paddler_agent_decision::decision_slots_minimum::DECISION_SLOTS_MINIMUM;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_tests::kev_0_8b_desired_state::kev_0_8b_desired_state;

use crate::state_transition_interrupted_by_shutdown::StateTransitionInterruptedByShutdown;

#[tokio::test(flavor = "multi_thread")]
async fn agent_shuts_down_cleanly_while_loading_a_decision_model() {
    StateTransitionInterruptedByShutdown {
        initial_desired_state: BalancerDesiredState::default(),
        next_desired_state: kev_0_8b_desired_state(),
        slot_count: DECISION_SLOTS_MINIMUM,
    }
    .shut_down_while_the_agent_applies_the_next_state()
    .await;
}
