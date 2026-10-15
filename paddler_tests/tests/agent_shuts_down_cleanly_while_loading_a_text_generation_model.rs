#![cfg(feature = "tests_that_use_llms")]

use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_tests::qwen3_desired_state::qwen3_desired_state;

use crate::state_transition_interrupted_by_shutdown::StateTransitionInterruptedByShutdown;

#[tokio::test(flavor = "multi_thread")]
async fn agent_shuts_down_cleanly_while_loading_a_text_generation_model() {
    StateTransitionInterruptedByShutdown {
        initial_desired_state: BalancerDesiredState::default(),
        next_desired_state: qwen3_desired_state(),
        slot_count: 1,
    }
    .shut_down_while_the_agent_applies_the_next_state()
    .await;
}
