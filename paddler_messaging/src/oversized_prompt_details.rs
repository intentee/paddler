use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OversizedPromptDetails {
    pub prompt_tokens: usize,
    pub sequence_context_size: u32,
}
