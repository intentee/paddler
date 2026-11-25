use serde::Deserialize;
use serde::Serialize;

use crate::generation_finish::GenerationFinish;
use crate::token_usage::TokenUsage;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GenerationSummary {
    pub finish: GenerationFinish,
    pub usage: TokenUsage,
}
