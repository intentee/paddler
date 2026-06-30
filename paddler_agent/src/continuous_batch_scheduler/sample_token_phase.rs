use llama_cpp_bindings::context::LlamaContext;

use crate::continuous_batch_active_request::ContinuousBatchActiveRequest;
use crate::continuous_batch_scheduler::sample_outcome::SampleOutcome;
use crate::sample_token_at_batch_index::sample_token_at_batch_index;
use crate::sampling_outcome::SamplingOutcome;

#[must_use]
pub fn sample_outcome_from_sampling(outcome: SamplingOutcome) -> SampleOutcome {
    match outcome {
        SamplingOutcome::Token(token) => SampleOutcome::Sampled(token),
        SamplingOutcome::AllCandidatesEliminated => SampleOutcome::AllCandidatesEliminated,
        SamplingOutcome::GrammarRejectedModelOutput(message) => {
            SampleOutcome::GrammarRejected(message)
        }
    }
}

pub struct SampleTokenPhase<'context> {
    pub context: &'context LlamaContext<'context>,
}

impl SampleTokenPhase<'_> {
    pub fn run(
        &self,
        request: &mut ContinuousBatchActiveRequest,
        batch_index: i32,
    ) -> SampleOutcome {
        sample_token_at_batch_index(
            self.context,
            batch_index,
            &mut request.chain,
            &mut request.grammar_sampler,
        )
        .map_or_else(
            |err| SampleOutcome::Failed(err.to_string()),
            sample_outcome_from_sampling,
        )
    }
}

#[cfg(test)]
mod tests {
    use std::mem::discriminant;

    use llama_cpp_bindings::token::LlamaToken;

    use super::sample_outcome_from_sampling;
    use crate::continuous_batch_scheduler::sample_outcome::SampleOutcome;
    use crate::sampling_outcome::SamplingOutcome;

    #[test]
    fn token_maps_to_sampled() {
        let outcome = sample_outcome_from_sampling(SamplingOutcome::Token(LlamaToken::new(7)));

        assert!(matches!(outcome, SampleOutcome::Sampled(token) if token == LlamaToken::new(7)));
    }

    #[test]
    fn all_candidates_eliminated_maps_to_all_candidates_eliminated() {
        let outcome = sample_outcome_from_sampling(SamplingOutcome::AllCandidatesEliminated);

        assert_eq!(
            discriminant(&outcome),
            discriminant(&SampleOutcome::AllCandidatesEliminated)
        );
    }

    #[test]
    fn grammar_rejected_model_output_maps_to_grammar_rejected() {
        let outcome = sample_outcome_from_sampling(SamplingOutcome::GrammarRejectedModelOutput(
            "nope".to_owned(),
        ));

        assert!(matches!(outcome, SampleOutcome::GrammarRejected(message) if message == "nope"));
    }
}
