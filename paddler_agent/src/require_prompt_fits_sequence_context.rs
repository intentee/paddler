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

    fn rejection_message(prompt_tokens: usize, sequence_context_size: u32) -> Result<(), String> {
        require_prompt_fits_sequence_context(prompt_tokens, sequence_context_size)
            .map_err(|rejection| rejection.to_string())
    }

    #[test]
    fn accepts_a_prompt_that_leaves_room_for_generated_tokens() {
        assert_eq!(rejection_message(8191, 8192), Ok(()));
    }

    #[test]
    fn rejects_a_prompt_that_fills_the_whole_sequence_context() {
        assert_eq!(
            rejection_message(8192, 8192),
            Err("prompt has 8192 tokens but each sequence holds 8192 tokens".to_owned())
        );
    }

    #[test]
    fn rejects_a_prompt_longer_than_the_sequence_context() {
        assert_eq!(
            rejection_message(9895, 8192),
            Err("prompt has 9895 tokens but each sequence holds 8192 tokens".to_owned())
        );
    }
}
