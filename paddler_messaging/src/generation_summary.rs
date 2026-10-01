use llama_cpp_bindings_types::TokenUsage;
use serde::Deserialize;
use serde::Serialize;

use crate::generation_finish::GenerationFinish;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GenerationSummary {
    pub finish: GenerationFinish,
    pub usage: TokenUsage,
}
