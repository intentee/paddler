use std::num::NonZeroU32;
use std::vec::IntoIter;

use llama_cpp_bindings::sampled_token::SampledToken;
use llama_cpp_bindings::sampled_token_section::SampledTokenSection;
use llama_cpp_bindings::token::LlamaToken;

use crate::constants::LAST_LOGITS_BATCH_POSITION;
use crate::continuous_batch_generation_step::ContinuousBatchGenerationStep;
use crate::continuous_batch_request_phase::ContinuousBatchRequestPhase;
use crate::continuous_batch_terminal_outcome::ContinuousBatchTerminalOutcome;
use crate::generating_contribution::GeneratingContribution;
use crate::ingesting_contribution::IngestingContribution;

pub struct ContinuousBatchRequestState {
    pub current_token_position: i32,
    pub last_outcome_section: SampledTokenSection,
    pub max_tokens: NonZeroU32,
    pub phase: ContinuousBatchRequestPhase,
    pub sampled_tokens: u64,
    pub unread_prompt_tokens: IntoIter<LlamaToken>,
}

impl ContinuousBatchRequestState {
    pub fn apply_generating_contribution(
        &mut self,
        &GeneratingContribution {
            batch_position,
            next_token_position,
            ..
        }: &GeneratingContribution,
    ) {
        self.phase =
            ContinuousBatchRequestPhase::Generating(ContinuousBatchGenerationStep::ReadyToSample {
                batch_position,
            });
        self.current_token_position = next_token_position;
    }

    pub fn apply_ingesting_contribution(
        &mut self,
        &IngestingContribution {
            chunk_size,
            is_last_chunk,
            last_batch_position,
            next_token_position,
            ..
        }: &IngestingContribution,
    ) {
        self.unread_prompt_tokens
            .by_ref()
            .take(chunk_size)
            .for_each(drop);
        self.current_token_position = next_token_position;

        if is_last_chunk {
            self.phase = ContinuousBatchRequestPhase::Generating(
                ContinuousBatchGenerationStep::ReadyToSample {
                    batch_position: last_batch_position,
                },
            );
        }
    }

    pub fn await_decode_of(&mut self, sampled_token: SampledToken) {
        self.phase = ContinuousBatchRequestPhase::Generating(
            ContinuousBatchGenerationStep::AwaitingDecode(sampled_token),
        );
    }

    pub fn begin_generating_after_multimodal_prompt(
        &mut self,
        next_position: i32,
        prompt_section: SampledTokenSection,
    ) {
        self.current_token_position = next_position;
        self.last_outcome_section = prompt_section;
        self.phase =
            ContinuousBatchRequestPhase::Generating(ContinuousBatchGenerationStep::ReadyToSample {
                batch_position: LAST_LOGITS_BATCH_POSITION,
            });
    }

    #[must_use]
    pub fn into_terminal_outcome(self) -> ContinuousBatchTerminalOutcome {
        match self.phase {
            ContinuousBatchRequestPhase::Completed(terminal_outcome) => terminal_outcome,
            ContinuousBatchRequestPhase::Generating(_)
            | ContinuousBatchRequestPhase::IngestingText
            | ContinuousBatchRequestPhase::IngestingMultimodal(_) => {
                ContinuousBatchTerminalOutcome::EmitNothing
            }
        }
    }

    pub fn mark_completed(&mut self, terminal_outcome: ContinuousBatchTerminalOutcome) {
        if matches!(self.phase, ContinuousBatchRequestPhase::Completed(_)) {
            return;
        }

        self.phase = ContinuousBatchRequestPhase::Completed(terminal_outcome);
    }

    pub const fn record_sampled_token(&mut self) {
        self.sampled_tokens = self.sampled_tokens.saturating_add(1);
    }

    #[must_use]
    pub fn remaining_prompt_tokens(&self) -> &[LlamaToken] {
        self.unread_prompt_tokens.as_slice()
    }
}

#[cfg(test)]
mod tests {
    use std::mem::discriminant;
    use std::num::NonZeroU32;

    use llama_cpp_bindings::sampled_token::SampledToken;
    use llama_cpp_bindings::sampled_token_section::SampledTokenSection;
    use llama_cpp_bindings::token::LlamaToken;
    use llama_cpp_bindings_types::token_usage::TokenUsage;

    use paddler_messaging::generated_token_result::GeneratedTokenResult;
    use paddler_messaging::generation_finish::GenerationFinish;
    use paddler_messaging::generation_summary::GenerationSummary;

    use super::ContinuousBatchRequestState;
    use crate::continuous_batch_generation_step::ContinuousBatchGenerationStep;
    use crate::continuous_batch_request_phase::ContinuousBatchRequestPhase;
    use crate::continuous_batch_terminal_outcome::ContinuousBatchTerminalOutcome;
    use crate::generating_contribution::GeneratingContribution;
    use crate::ingesting_contribution::IngestingContribution;

    fn ingesting_state(prompt_token_count: usize) -> ContinuousBatchRequestState {
        ContinuousBatchRequestState {
            current_token_position: 0,
            last_outcome_section: SampledTokenSection::Content,
            max_tokens: NonZeroU32::new(64).unwrap(),
            phase: ContinuousBatchRequestPhase::IngestingText,
            sampled_tokens: 0,
            unread_prompt_tokens: vec![LlamaToken::new(1); prompt_token_count].into_iter(),
        }
    }

    const fn done() -> GeneratedTokenResult {
        GeneratedTokenResult::Done(GenerationSummary {
            finish: GenerationFinish::EndOfGeneration,
            usage: TokenUsage::new(),
        })
    }

    #[test]
    fn the_first_terminal_outcome_wins() {
        let mut state = ingesting_state(0);

        state.mark_completed(ContinuousBatchTerminalOutcome::EmitToClient(
            GeneratedTokenResult::SamplerError("sampler failed".to_owned()),
        ));
        state.mark_completed(ContinuousBatchTerminalOutcome::EmitToClient(done()));

        assert!(
            matches!(
                state.phase,
                ContinuousBatchRequestPhase::Completed(
                    ContinuousBatchTerminalOutcome::EmitToClient(
                        GeneratedTokenResult::SamplerError(ref message)
                    )
                ) if message == "sampler failed"
            ),
            "a later stop must not overwrite the real failure that ended the request"
        );
    }

    #[test]
    fn remaining_prompt_tokens_skips_already_ingested_tokens() {
        let mut state = ingesting_state(0);
        state.unread_prompt_tokens = vec![
            LlamaToken::new(1),
            LlamaToken::new(2),
            LlamaToken::new(3),
            LlamaToken::new(4),
            LlamaToken::new(5),
        ]
        .into_iter();

        state.apply_ingesting_contribution(&IngestingContribution {
            request_index: 0,
            chunk_size: 2,
            is_last_chunk: false,
            last_batch_position: 1,
            next_token_position: 2,
        });

        assert_eq!(
            state.remaining_prompt_tokens(),
            [LlamaToken::new(3), LlamaToken::new(4), LlamaToken::new(5)]
        );
    }

    #[test]
    fn applying_a_generating_contribution_readies_sampling_and_advances_position() {
        let mut state = ingesting_state(0);
        state.current_token_position = 7;
        state.await_decode_of(SampledToken::Content(LlamaToken::new(9)));

        state.apply_generating_contribution(&GeneratingContribution {
            request_index: 0,
            batch_position: 3,
            next_token_position: 8,
        });

        assert!(matches!(
            state.phase,
            ContinuousBatchRequestPhase::Generating(ContinuousBatchGenerationStep::ReadyToSample {
                batch_position
            }) if batch_position == 3
        ));
        assert_eq!(state.current_token_position, 8);
    }

    #[test]
    fn applying_a_non_final_ingesting_chunk_advances_without_transitioning() {
        let mut state = ingesting_state(10);

        state.apply_ingesting_contribution(&IngestingContribution {
            request_index: 0,
            chunk_size: 4,
            is_last_chunk: false,
            last_batch_position: 99,
            next_token_position: 4,
        });

        assert_eq!(state.remaining_prompt_tokens().len(), 6);
        assert_eq!(state.current_token_position, 4);
        assert_eq!(
            discriminant(&state.phase),
            discriminant(&ContinuousBatchRequestPhase::IngestingText)
        );
    }

    #[test]
    fn applying_the_final_ingesting_chunk_transitions_to_generating() {
        let mut state = ingesting_state(2);
        state.current_token_position = 4;

        state.apply_ingesting_contribution(&IngestingContribution {
            request_index: 0,
            chunk_size: 2,
            is_last_chunk: true,
            last_batch_position: 41,
            next_token_position: 6,
        });

        assert_eq!(state.remaining_prompt_tokens(), []);
        assert_eq!(state.current_token_position, 6);
        assert!(matches!(
            state.phase,
            ContinuousBatchRequestPhase::Generating(ContinuousBatchGenerationStep::ReadyToSample {
                batch_position
            }) if batch_position == 41
        ));
    }

    #[test]
    fn a_sampled_token_awaits_its_decode() {
        let mut state = ingesting_state(0);

        state.await_decode_of(SampledToken::Content(LlamaToken::new(5)));

        assert!(matches!(
            state.phase,
            ContinuousBatchRequestPhase::Generating(ContinuousBatchGenerationStep::AwaitingDecode(
                SampledToken::Content(token)
            )) if token == LlamaToken::new(5)
        ));
    }

    #[test]
    fn marking_a_generating_request_completed_carries_the_terminal_outcome() {
        let mut state = ingesting_state(0);
        state.phase =
            ContinuousBatchRequestPhase::Generating(ContinuousBatchGenerationStep::ReadyToSample {
                batch_position: 2,
            });

        state.mark_completed(ContinuousBatchTerminalOutcome::EmitToClient(done()));

        assert_eq!(
            state.into_terminal_outcome(),
            ContinuousBatchTerminalOutcome::EmitToClient(done())
        );
    }

    #[test]
    fn a_request_torn_down_before_it_completed_reports_nothing_to_the_client() {
        let state = ingesting_state(0);

        assert_eq!(
            state.into_terminal_outcome(),
            ContinuousBatchTerminalOutcome::EmitNothing
        );
    }
}
