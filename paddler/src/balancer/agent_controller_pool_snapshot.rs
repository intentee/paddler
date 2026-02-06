use serde::Deserialize;
use serde::Serialize;

use crate::balancer::agent_controller_snapshot::AgentControllerSnapshot;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AgentControllerPoolSnapshot {
    pub agents: Vec<AgentControllerSnapshot>,
}
