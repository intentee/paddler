use serde::Deserialize;
use serde::Serialize;

use llama_cpp_bindings_types::TokenUsage;

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GenerationSummary {
    pub usage: TokenUsage,
}
