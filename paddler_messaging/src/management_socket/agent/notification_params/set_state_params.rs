use crate::agent_desired_state::AgentDesiredState;
use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SetStateParams {
    pub desired_state: AgentDesiredState,
}
