use llama_cpp_bindings::context::LlamaContext;

use crate::agent::continuous_batch_active_request::ContinuousBatchActiveRequest;
use crate::agent::continuous_batch_scheduler::sample_outcome::SampleOutcome;
use crate::agent::sample_token_at_batch_index::sample_token_at_batch_index;
use crate::agent::sampling_outcome::SamplingOutcome;

pub struct SampleTokenPhase<'context> {
    pub context: &'context LlamaContext<'context>,
}

impl SampleTokenPhase<'_> {
    pub fn run(
        &self,
        request: &mut ContinuousBatchActiveRequest,
        batch_index: i32,
    ) -> SampleOutcome {
        match sample_token_at_batch_index(
            self.context,
            batch_index,
            &mut request.chain,
            &mut request.grammar_sampler,
        ) {
            Ok(SamplingOutcome::Token(token)) => SampleOutcome::Sampled(token),
            Ok(SamplingOutcome::AllCandidatesEliminated) => SampleOutcome::AllCandidatesEliminated,
            Ok(SamplingOutcome::GrammarRejectedModelOutput(message)) => {
                SampleOutcome::GrammarRejected(message)
            }
            Err(err) => SampleOutcome::Failed(err.to_string()),
        }
    }
}
