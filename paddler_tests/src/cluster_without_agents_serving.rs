use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_mode::InferenceMode;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;

#[must_use]
pub fn cluster_without_agents_serving(inference_mode: InferenceMode) -> ClusterParams {
    ClusterParams {
        agents: Vec::new(),
        desired_state: ClusterDesiredState::Apply(Box::new(BalancerDesiredState {
            inference_mode,
            ..BalancerDesiredState::default()
        })),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    }
}
