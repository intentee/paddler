use std::future::Future;

use anyhow::Result;

use paddler_cluster_harness::agent_config::AgentConfig;
use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_messaging::constants::DECISION_SLOTS_MINIMUM;

use crate::decision_desired_state::DecisionDesiredState;
use crate::start_cluster::start_cluster;

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
    /// Returns the error of [`start_cluster`] when the cluster cannot start.
    pub fn start_cluster(self) -> impl Future<Output = Result<Cluster>> {
        start_cluster(self.into_cluster_params())
    }
}

impl Default for DecisionClusterParams {
    fn default() -> Self {
        Self {
            agents: AgentConfig::uniform(1, DECISION_SLOTS_MINIMUM.get()),
            desired_state: DecisionDesiredState::default(),
            wait_for_slots_ready: true,
        }
    }
}
