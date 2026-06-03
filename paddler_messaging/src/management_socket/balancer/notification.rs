use serde::Deserialize;
use serde::Serialize;

use super::notification_params::RegisterAgentParams;
use super::notification_params::UpdateAgentStatusParams;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub enum Notification {
    DeregisterAgent,
    RegisterAgent(RegisterAgentParams),
    UpdateAgentStatus(UpdateAgentStatusParams),
}
