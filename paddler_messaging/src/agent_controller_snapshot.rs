use serde::Deserialize;
use serde::Serialize;

use crate::agent_status::AgentStatus;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AgentControllerSnapshot {
    pub id: String,
    pub name: Option<String>,
    pub slots_processing: u64,
    pub status: AgentStatus,
}
