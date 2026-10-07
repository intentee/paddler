use paddler_messaging::oversized_prompt_details::OversizedPromptDetails;

use crate::text_generation_error::TextGenerationError;

pub const fn require_prompt_fits_sequence_context(
    prompt_tokens: usize,
    sequence_context_size: u32,
) -> Result<(), TextGenerationError> {
    if prompt_tokens < sequence_context_size as usize {
        Ok(())
    } else {
        Err(TextGenerationError::PromptExceedsContextSize {
            details: OversizedPromptDetails {
                prompt_tokens,
                sequence_context_size,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use paddler_messaging::oversized_prompt_details::OversizedPromptDetails;

    use super::require_prompt_fits_sequence_context;
    use crate::text_generation_error::TextGenerationError;

    fn rejects_as_oversized(prompt_tokens: usize, sequence_context_size: u32) -> bool {
        matches!(
            require_prompt_fits_sequence_context(prompt_tokens, sequence_context_size),
            Err(TextGenerationError::PromptExceedsContextSize { details })
                if details == OversizedPromptDetails {
                    prompt_tokens,
                    sequence_context_size,
                }
        )
    }

    #[test]
    fn accepts_a_prompt_that_leaves_room_for_generated_tokens() {
        assert!(require_prompt_fits_sequence_context(8191, 8192).is_ok());
    }

    #[test]
    fn rejects_a_prompt_that_fills_the_whole_sequence_context() {
        assert!(rejects_as_oversized(8192, 8192));
    }

    #[test]
    fn rejects_a_prompt_longer_than_the_sequence_context() {
        assert!(rejects_as_oversized(9895, 8192));
    }
}
