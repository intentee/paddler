use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SlotsInsufficientForDecisionsParams {
    pub desired_slots: u16,
    pub required_slots: u16,
}
