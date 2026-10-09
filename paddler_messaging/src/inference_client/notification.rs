use serde::Deserialize;
use serde::Serialize;

use crate::inference_mode::InferenceMode;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub enum Notification {
    ClusterInferenceMode(InferenceMode),
}
