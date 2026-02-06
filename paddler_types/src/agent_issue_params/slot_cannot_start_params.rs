use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SlotCannotStartParams {
    pub error: String,
    pub slot_index: u32,
}
