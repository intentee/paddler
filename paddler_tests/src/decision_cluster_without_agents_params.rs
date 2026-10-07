use paddler_messaging::inference_mode::InferenceMode;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;

#[must_use]
pub fn decision_cluster_without_agents_params() -> ClusterParams {
    ClusterParams {
        agents: Vec::new(),
        desired_state: ClusterDesiredState::KeepStored(InferenceMode::Decision),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    }
}
