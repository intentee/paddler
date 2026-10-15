use serde::Deserialize;
use serde::Serialize;

use crate::agent_desired_model::AgentDesiredModel;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionSettings {
    pub pointer_head: AgentDesiredModel,
}
