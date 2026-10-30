#![cfg(feature = "tests_that_use_llms")]

use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::decision_slots_minimum::DECISION_SLOTS_MINIMUM;
use paddler_tests::kev_checkpoint::KevCheckpoint;

use crate::shutdown_during_state_transition::ShutdownDuringStateTransition;

#[tokio::test(flavor = "multi_thread")]
async fn agent_shuts_down_cleanly_while_starting_to_serve_decisions() {
    ShutdownDuringStateTransition {
        initial_desired_state: BalancerDesiredState::default(),
        next_desired_state: KevCheckpoint::Kev0_8b
            .decision_desired_state()
            .into_balancer_desired_state()
            .expect("the kev desired state must name absolute paths"),
        slot_count: DECISION_SLOTS_MINIMUM.get(),
    }
    .shut_down_while_the_agent_applies_the_next_state()
    .await;
}
