use anyhow::Result;

use paddler_model_card_fixture::ministral_3_3b_reasoning::ministral_3_3b_reasoning;

use crate::agent_config::AgentConfig;
use crate::cluster::Cluster;
use crate::cluster_desired_state::ClusterDesiredState;
use crate::cluster_params::ClusterParams;

pub struct Ministral3ClusterParams {
    pub agents: Vec<AgentConfig>,
}

impl Ministral3ClusterParams {
    /// # Errors
    ///
    /// Returns the error of [`Cluster::start_in_process`] when the cluster serving Ministral 3 cannot start.
    pub async fn start_cluster(self) -> Result<Cluster> {
        let Self { agents } = self;

        Cluster::start_in_process(ClusterParams {
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
