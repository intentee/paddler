use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TokenUsage {
    pub prompt_tokens: u64,
    pub cached_prompt_tokens: u64,
    pub input_image_tokens: u64,
    pub input_audio_tokens: u64,
    pub content_tokens: u64,
    pub reasoning_tokens: u64,
    pub tool_call_tokens: u64,
    pub undeterminable_tokens: u64,
}

impl TokenUsage {
    #[must_use]
    pub const fn completion_tokens(&self) -> u64 {
        self.content_tokens
            + self.reasoning_tokens
            + self.tool_call_tokens
            + self.undeterminable_tokens
    }

    #[must_use]
    pub const fn total_tokens(&self) -> u64 {
        self.prompt_tokens + self.completion_tokens()
    }
}
