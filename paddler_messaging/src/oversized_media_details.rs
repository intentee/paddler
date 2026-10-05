use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OversizedMediaDetails {
    pub media_tokens: usize,
    pub micro_batch_tokens: u32,
}
