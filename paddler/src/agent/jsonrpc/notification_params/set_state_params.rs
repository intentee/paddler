use serde::Deserialize;
use serde::Serialize;

use crate::agent_desired_state::AgentDesiredState;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SetStateParams {
    pub desired_state: AgentDesiredState,
}
