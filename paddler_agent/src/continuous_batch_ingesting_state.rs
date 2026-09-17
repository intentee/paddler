use llama_cpp_bindings::token::LlamaToken;

#[derive(Debug)]
pub struct ContinuousBatchIngestingState {
    pub prompt_tokens: Vec<LlamaToken>,
    pub prompt_tokens_ingested: usize,
    pub prompt_tokens_staged_into_classifier: usize,
}

impl ContinuousBatchIngestingState {
    #[must_use]
    pub const fn new(prompt_tokens: Vec<LlamaToken>) -> Self {
        Self {
            prompt_tokens,
            prompt_tokens_ingested: 0,
            prompt_tokens_staged_into_classifier: 0,
        }
    }

    #[must_use]
    pub fn remaining_prompt_tokens(&self) -> &[LlamaToken] {
        &self.prompt_tokens[self.prompt_tokens_ingested..]
    }

    #[must_use]
    pub const fn is_already_staged_into_classifier(&self, absolute_token_index: usize) -> bool {
        absolute_token_index < self.prompt_tokens_staged_into_classifier
    }
}

#[cfg(test)]
mod tests {
    use llama_cpp_bindings::token::LlamaToken;

    use super::ContinuousBatchIngestingState;

    fn state_with(prompt_token_count: usize) -> ContinuousBatchIngestingState {
        ContinuousBatchIngestingState::new(vec![LlamaToken::new(1); prompt_token_count])
    }

    #[test]
    fn a_fresh_state_has_ingested_and_staged_nothing() {
        let state = state_with(4);

        assert_eq!(state.prompt_tokens_ingested, 0);
        assert_eq!(state.prompt_tokens_staged_into_classifier, 0);
        assert_eq!(state.remaining_prompt_tokens().len(), 4);
    }

    #[test]
    fn remaining_prompt_tokens_skips_already_ingested_tokens() {
        let mut state = state_with(5);
        state.prompt_tokens_ingested = 2;

        assert_eq!(state.remaining_prompt_tokens().len(), 3);
    }

    #[test]
    fn tokens_below_the_staging_high_water_mark_are_already_in_the_classifier() {
        let mut state = state_with(5);
        state.prompt_tokens_staged_into_classifier = 3;

        assert!(state.is_already_staged_into_classifier(0));
        assert!(state.is_already_staged_into_classifier(2));
        assert!(!state.is_already_staged_into_classifier(3));
        assert!(!state.is_already_staged_into_classifier(4));
    }
}
