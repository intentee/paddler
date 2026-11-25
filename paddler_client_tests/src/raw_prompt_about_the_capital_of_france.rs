use std::num::NonZeroU32;

use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;

#[must_use]
pub fn raw_prompt_about_the_capital_of_france(
    max_tokens: NonZeroU32,
) -> ContinueFromRawPromptParams {
    ContinueFromRawPromptParams {
        grammar: None,
        max_tokens,
        raw_prompt: "The capital of France is".to_owned(),
    }
}
