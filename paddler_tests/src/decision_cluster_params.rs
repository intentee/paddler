use paddler_messaging::decision_slots_minimum::DECISION_SLOTS_MINIMUM;
use paddler_model_source::model_source_error::ModelSourceError;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;

use crate::decision_desired_state::DecisionDesiredState;

pub struct DecisionClusterParams {
    pub agents: Vec<AgentConfig>,
    pub desired_state: DecisionDesiredState,
    pub wait_for_slots_ready: bool,
}

impl DecisionClusterParams {
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
