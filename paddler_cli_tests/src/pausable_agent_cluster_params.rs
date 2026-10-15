use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;

pub struct PausableAgentClusterParams {
    pub binary_path: String,
    pub desired_state: ClusterDesiredState,
    pub expected_slots_total: u16,
    pub slot_count: u16,
}
