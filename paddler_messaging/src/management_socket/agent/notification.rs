use serde::Deserialize;
use serde::Serialize;

use super::notification_params::set_state_params::SetStateParams;
use super::notification_params::version_params::VersionParams;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub enum Notification {
    SetState(Box<SetStateParams>),
    StopRespondingTo(String),
    Version(VersionParams),
}
