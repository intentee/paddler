#![cfg(feature = "tests_that_use_llms")]

use paddler_agent_tests::shutdown_during_state_transition::ShutdownDuringStateTransition;
use paddler_balancer_desired_state_fixture::decision_desired_state::DecisionDesiredState;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::constants::DECISION_SLOTS_MINIMUM;

#[tokio::test(flavor = "multi_thread")]
async fn agent_shuts_down_cleanly_while_starting_to_serve_decisions() {
    ShutdownDuringStateTransition {
        initial_desired_state: BalancerDesiredState::default(),
        next_desired_state: DecisionDesiredState::default().into_balancer_desired_state(),
        slot_count: DECISION_SLOTS_MINIMUM.get(),
    }
    .shut_down_while_the_agent_applies_the_next_state()
    .await
    .expect("the cluster must shut down while its agent applies the next state");
}
