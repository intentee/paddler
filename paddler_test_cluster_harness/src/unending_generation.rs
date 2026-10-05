use std::num::NonZeroU32;

use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;

use crate::unending_grammar::UNENDING_GENERATION_CYCLE;
use crate::unending_grammar::unending_grammar;

#[must_use]
pub fn unending_generation() -> ContinueFromRawPromptParams {
    ContinueFromRawPromptParams {
        grammar: Some(unending_grammar()),
        max_tokens: NonZeroU32::MAX,
        raw_prompt: format!("Keep repeating: {UNENDING_GENERATION_CYCLE}"),
    }
}
