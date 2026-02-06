use serde::Deserialize;
use serde::Serialize;

use super::notification_params::SetStateParams;
use super::notification_params::VersionParams;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub enum Notification {
    SetState(SetStateParams),
    StopRespondingTo(String),
    Version(VersionParams),
}
