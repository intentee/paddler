use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentIdPathParams {
    pub agent_id: String,
}

impl AgentIdPathParams {
    pub const ROUTE_SEGMENT: &str = "{agent_id}";
}
