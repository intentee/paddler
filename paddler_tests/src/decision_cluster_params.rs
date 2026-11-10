use std::future::Future;

use anyhow::Result;
use futures_util::TryFutureExt as _;
use futures_util::future::ready;

use paddler_messaging::constants::DECISION_SLOTS_MINIMUM;
use paddler_model_source::model_source_error::ModelSourceError;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;

use crate::decision_desired_state::DecisionDesiredState;
use crate::start_cluster::start_cluster;

pub struct DecisionClusterParams {
    pub agents: Vec<AgentConfig>,
    pub desired_state: DecisionDesiredState,
    pub wait_for_slots_ready: bool,
}

impl DecisionClusterParams {
    /// # Errors
    ///
    /// Returns [`ModelSourceError::LocalModelPathNotAbsolute`] when the backbone or the pointer head lies at a relative path, and [`ModelSourceError::LocalModelPathClimbsToParentDirectory`] when either path climbs to a parent directory.
    pub fn into_cluster_params(self) -> Result<ClusterParams, ModelSourceError> {
        let Self {
            agents,
            desired_state,
            wait_for_slots_ready,
        } = self;

        Ok(ClusterParams {
            agents,
            desired_state: ClusterDesiredState::Apply(Box::new(
                desired_state.into_balancer_desired_state()?,
            )),
            wait_for_slots_ready,
            ..ClusterParams::default()
        })
    }

    /// # Errors
    ///
    /// Returns the [`ModelSourceError`] of [`Self::into_cluster_params`] when the desired state names a path that cannot be served, and the error of [`start_cluster`] when the cluster cannot start.
    pub fn start_cluster(self) -> impl Future<Output = Result<Cluster>> {
        ready(self.into_cluster_params())
            .err_into()
            .and_then(start_cluster)
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
