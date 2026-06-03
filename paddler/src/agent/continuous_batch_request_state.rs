use anyhow::Context as _;
use anyhow::Result;
use llama_cpp_bindings::SampledToken;
use llama_cpp_bindings::token::LlamaToken;

use crate::agent::continuous_batch_request_phase::ContinuousBatchRequestPhase;

pub struct ContinuousBatchRequestState {
    pub current_token_position: i32,
    pub i_batch: Option<i32>,
    pub max_tokens: i32,
    pub pending_sampled_token: Option<SampledToken>,
    pub phase: ContinuousBatchRequestPhase,
    pub prompt_tokens: Vec<LlamaToken>,
    pub prompt_tokens_ingested: usize,
    pub sequence_id: i32,
}

impl ContinuousBatchRequestState {
    #[must_use]
    pub fn remaining_prompt_tokens(&self) -> &[LlamaToken] {
        &self.prompt_tokens[self.prompt_tokens_ingested..]
    }

    pub const fn apply_generating_contribution(&mut self, batch_position: i32) {
        self.pending_sampled_token = None;
        self.i_batch = Some(batch_position);
        self.current_token_position += 1;
    }

    pub fn apply_ingesting_contribution(
        &mut self,
        chunk_size: usize,
        is_last_chunk: bool,
        last_batch_position: i32,
    ) -> Result<()> {
        self.prompt_tokens_ingested += chunk_size;
        self.current_token_position +=
            i32::try_from(chunk_size).context("chunk size does not fit in i32")?;

        if is_last_chunk {
            self.i_batch = Some(last_batch_position);
            self.phase = ContinuousBatchRequestPhase::Generating;
        }

        Ok(())
    }

    pub const fn store_pending_token(&mut self, token: SampledToken) {
        self.pending_sampled_token = Some(token);
    }

    pub const fn mark_completed(&mut self) {
        self.i_batch = None;
        self.phase = ContinuousBatchRequestPhase::Completed;
    }
}

#[cfg(test)]
mod tests {
    use llama_cpp_bindings::SampledToken;
    use llama_cpp_bindings::token::LlamaToken;

    use super::ContinuousBatchRequestState;
    use crate::agent::continuous_batch_request_phase::ContinuousBatchRequestPhase;

    fn ingesting_state(prompt_token_count: usize) -> ContinuousBatchRequestState {
        ContinuousBatchRequestState {
            current_token_position: 0,
            i_batch: None,
            max_tokens: 64,
            pending_sampled_token: None,
            phase: ContinuousBatchRequestPhase::Ingesting,
            prompt_tokens: vec![LlamaToken::new(1); prompt_token_count],
            prompt_tokens_ingested: 0,
            sequence_id: 0,
        }
    }

    #[test]
    fn remaining_prompt_tokens_skips_already_ingested_tokens() {
        let mut state = ingesting_state(5);
        state.prompt_tokens_ingested = 2;

        assert_eq!(state.remaining_prompt_tokens().len(), 3);
    }

    #[test]
    fn applying_a_generating_contribution_clears_pending_and_advances_position() {
        let mut state = ingesting_state(0);
        state.current_token_position = 7;
        state.pending_sampled_token = Some(SampledToken::Content(LlamaToken::new(9)));

        state.apply_generating_contribution(3);

        assert!(state.pending_sampled_token.is_none());
        assert_eq!(state.i_batch, Some(3));
        assert_eq!(state.current_token_position, 8);
    }

    #[test]
    fn applying_a_non_final_ingesting_chunk_advances_without_transitioning() {
        let mut state = ingesting_state(10);

        state.apply_ingesting_contribution(4, false, 99).unwrap();

        assert_eq!(state.prompt_tokens_ingested, 4);
        assert_eq!(state.current_token_position, 4);
        assert_eq!(state.i_batch, None);
        assert!(matches!(
            state.phase,
            ContinuousBatchRequestPhase::Ingesting
        ));
    }

    #[test]
    fn applying_the_final_ingesting_chunk_transitions_to_generating() {
        let mut state = ingesting_state(6);
        state.prompt_tokens_ingested = 4;
        state.current_token_position = 4;

        state.apply_ingesting_contribution(2, true, 41).unwrap();

        assert_eq!(state.prompt_tokens_ingested, 6);
        assert_eq!(state.current_token_position, 6);
        assert_eq!(state.i_batch, Some(41));
        assert!(matches!(
            state.phase,
            ContinuousBatchRequestPhase::Generating
        ));
    }

    #[test]
    fn applying_an_ingesting_chunk_too_large_for_i32_is_an_error() {
        let mut state = ingesting_state(0);

        let result = state.apply_ingesting_contribution(usize::MAX, false, 0);

        assert!(result.is_err());
    }

    #[test]
    fn storing_a_pending_token_records_it() {
        let mut state = ingesting_state(0);

        state.store_pending_token(SampledToken::Content(LlamaToken::new(5)));

        assert!(matches!(
            state.pending_sampled_token,
            Some(SampledToken::Content(token)) if token == LlamaToken::new(5)
        ));
    }

    #[test]
    fn marking_completed_clears_batch_index_and_sets_completed_phase() {
        let mut state = ingesting_state(0);
        state.i_batch = Some(2);
        state.phase = ContinuousBatchRequestPhase::Generating;

        state.mark_completed();

        assert_eq!(state.i_batch, None);
        assert!(matches!(
            state.phase,
            ContinuousBatchRequestPhase::Completed
        ));
    }
}
