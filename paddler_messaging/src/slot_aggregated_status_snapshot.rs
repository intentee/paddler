use serde::Deserialize;
use serde::Serialize;

use crate::agent_status::AgentStatus;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SlotAggregatedStatusSnapshot {
    pub status: AgentStatus,
    pub version: u64,
}
