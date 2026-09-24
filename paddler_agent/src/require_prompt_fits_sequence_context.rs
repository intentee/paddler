use paddler_messaging::oversized_prompt_details::OversizedPromptDetails;

use crate::generation_request_rejection::GenerationRequestRejection;

pub const fn require_prompt_fits_sequence_context(
    prompt_tokens: usize,
    sequence_context_size: u32,
) -> Result<(), GenerationRequestRejection> {
    if prompt_tokens < sequence_context_size as usize {
        Ok(())
    } else {
        Err(GenerationRequestRejection::PromptExceedsContextSize {
            details: OversizedPromptDetails {
                prompt_tokens,
                sequence_context_size,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::require_prompt_fits_sequence_context;
    use crate::generation_request_rejection::GenerationRequestRejection;

    #[test]
    fn accepts_a_prompt_that_leaves_room_for_generated_tokens() {
        assert!(require_prompt_fits_sequence_context(8191, 8192).is_ok());
    }

    #[test]
    fn rejects_a_prompt_that_fills_the_whole_sequence_context() {
        assert!(matches!(
            require_prompt_fits_sequence_context(8192, 8192),
            Err(GenerationRequestRejection::PromptExceedsContextSize { details })
                if details.prompt_tokens == 8192 && details.sequence_context_size == 8192
        ));
    }

    #[test]
    fn rejects_a_prompt_longer_than_the_sequence_context() {
        assert!(matches!(
            require_prompt_fits_sequence_context(9895, 8192),
            Err(GenerationRequestRejection::PromptExceedsContextSize { details })
                if details.prompt_tokens == 9895
        ));
    }
}
