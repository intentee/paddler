use serde::Deserialize;
use serde::Serialize;

use crate::token_usage::TokenUsage;

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GenerationSummary {
    pub usage: TokenUsage,
}
