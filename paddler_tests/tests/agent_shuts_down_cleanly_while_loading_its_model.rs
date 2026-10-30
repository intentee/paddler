#![cfg(feature = "tests_that_use_llms")]

use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_tests::qwen3_desired_state::qwen3_desired_state;

use crate::shutdown_during_state_transition::ShutdownDuringStateTransition;

#[tokio::test(flavor = "multi_thread")]
async fn agent_shuts_down_cleanly_while_loading_its_model() {
    ShutdownDuringStateTransition {
        initial_desired_state: BalancerDesiredState::default(),
        next_desired_state: qwen3_desired_state(),
        slot_count: 1,
    }
    .shut_down_while_the_agent_applies_the_next_state()
    .await;
}
