use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TokenUsage {
    #[serde(rename = "prompt_tokens")]
    pub prompt: u64,
    #[serde(rename = "cached_prompt_tokens")]
    pub cached_prompt: u64,
    #[serde(rename = "input_image_tokens")]
    pub input_image: u64,
    #[serde(rename = "input_audio_tokens")]
    pub input_audio: u64,
    #[serde(rename = "content_tokens")]
    pub content: u64,
    #[serde(rename = "reasoning_tokens")]
    pub reasoning: u64,
    #[serde(rename = "tool_call_tokens")]
    pub tool_call: u64,
    #[serde(rename = "undeterminable_tokens")]
    pub undeterminable: u64,
}

impl TokenUsage {
    #[must_use]
    pub const fn completion_tokens(&self) -> u64 {
        self.content
            .saturating_add(self.reasoning)
            .saturating_add(self.tool_call)
            .saturating_add(self.undeterminable)
    }

    #[must_use]
    pub const fn total_tokens(&self) -> u64 {
        self.prompt.saturating_add(self.completion_tokens())
    }
}
