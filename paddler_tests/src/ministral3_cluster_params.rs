use anyhow::Result;

use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::model_card::ministral_3_3b_reasoning::ministral_3_3b_reasoning;

use crate::start_cluster::start_cluster;

pub struct Ministral3ClusterParams {
    pub agents: Vec<AgentConfig>,
}

impl Ministral3ClusterParams {
    /// # Errors
    ///
    /// Returns the error of [`start_cluster`] when the cluster serving Ministral 3 cannot start.
    pub async fn start_cluster(self) -> Result<Cluster> {
        let Self { agents } = self;

        start_cluster(ClusterParams {
            agents,
            desired_state: ClusterDesiredState::Apply(Box::new(
                ministral_3_3b_reasoning().into_desired_state(),
            )),
            wait_for_slots_ready: true,
            ..ClusterParams::default()
        })
        .await
    }
}

impl Default for Ministral3ClusterParams {
    fn default() -> Self {
        Self {
            agents: AgentConfig::uniform(1, 1),
        }
    }
}
