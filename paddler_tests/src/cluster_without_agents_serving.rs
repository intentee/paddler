use paddler_messaging::inference_mode::InferenceMode;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;

use crate::desired_state_serving::desired_state_serving;

#[must_use]
pub fn cluster_without_agents_serving(inference_mode: InferenceMode) -> ClusterParams {
    ClusterParams {
        agents: Vec::new(),
        desired_state: ClusterDesiredState::Apply(Box::new(desired_state_serving(inference_mode))),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    }
}
