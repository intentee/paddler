use paddler_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;

#[must_use]
pub fn qwen3_desired_state() -> BalancerDesiredState {
    qwen3_0_6b().into_desired_state()
}
