use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OversizedDecisionDetails {
    pub context_size: u32,
    pub required_tokens: usize,
}
