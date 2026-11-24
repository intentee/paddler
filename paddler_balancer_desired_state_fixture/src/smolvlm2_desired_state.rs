use paddler_cluster_harness::model_card::smolvlm2_256m::smolvlm2_256m;
use paddler_cluster_harness::model_card::smolvlm2_256m_mmproj::smolvlm2_256m_mmproj;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;

#[must_use]
pub fn smolvlm2_desired_state() -> BalancerDesiredState {
    smolvlm2_256m().into_desired_state_with_multimodal_projection(smolvlm2_256m_mmproj())
}
