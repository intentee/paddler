use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;

use crate::desired_state_serving_decisions::desired_state_serving_decisions;

#[must_use]
pub fn cluster_without_agents_serving_decisions() -> ClusterParams {
    ClusterParams {
        desired_state: ClusterDesiredState::Apply(Box::new(desired_state_serving_decisions())),
        ..ClusterParams::without_agents()
    }
}
