use std::future::Future;

use anyhow::Result;

use paddler_balancer_desired_state_fixture::decision_desired_state::DecisionDesiredState;
use paddler_messaging::constants::DECISION_SLOTS_WITH_A_QUESTION_LANE;

use crate::agent_config::AgentConfig;
use crate::cluster::Cluster;
use crate::cluster_desired_state::ClusterDesiredState;
use crate::cluster_params::ClusterParams;

pub struct DecisionClusterParams {
    pub agents: Vec<AgentConfig>,
    pub desired_state: DecisionDesiredState,
    pub wait_for_slots_ready: bool,
}

impl DecisionClusterParams {
    #[must_use]
    pub fn into_cluster_params(self) -> ClusterParams {
        let Self {
            agents,
            desired_state,
            wait_for_slots_ready,
        } = self;

        ClusterParams {
            agents,
            desired_state: ClusterDesiredState::Apply(Box::new(
                desired_state.into_balancer_desired_state(),
            )),
            wait_for_slots_ready,
            ..ClusterParams::default()
        }
    }

    /// # Errors
    ///
    /// Returns the error of [`Cluster::start_in_process`] when the cluster cannot start.
    pub fn start_cluster(self) -> impl Future<Output = Result<Cluster>> {
        Cluster::start_in_process(self.into_cluster_params())
    }
}

impl Default for DecisionClusterParams {
    fn default() -> Self {
        Self {
            agents: AgentConfig::uniform(1, DECISION_SLOTS_WITH_A_QUESTION_LANE.get()),
            desired_state: DecisionDesiredState::default(),
            wait_for_slots_ready: true,
        }
    }
}
