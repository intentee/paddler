use serde::Deserialize;
use serde::Serialize;

use crate::inference_capability::InferenceCapability;

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SlotCannotStartParams {
    pub error: String,
    pub inference_capability: InferenceCapability,
    pub slot_index: u32,
}
