use serde::Deserialize;
use serde::Serialize;

use crate::served_decision_settings::ServedDecisionSettings;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub enum AgentDecisionsSettings {
    NotServed,
    Served(ServedDecisionSettings),
}
