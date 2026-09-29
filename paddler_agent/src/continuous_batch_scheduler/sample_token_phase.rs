use llama_cpp_bindings::context::LlamaContext;
use llama_cpp_bindings::token::data_array::LlamaTokenDataArray;

use crate::continuous_batch_active_request::ContinuousBatchActiveRequest;
use crate::continuous_batch_scheduler::sample_outcome::SampleOutcome;
use crate::sampling_outcome::SamplingOutcome;

pub struct SampleTokenPhase<'context> {
    pub context: &'context LlamaContext<'context>,
}

impl SampleTokenPhase<'_> {
    pub fn run(
        &self,
        request: &mut ContinuousBatchActiveRequest,
        batch_index: i32,
        candidates: &mut LlamaTokenDataArray,
    ) -> SampleOutcome {
        match request
            .token_sampling
            .sample(self.context, batch_index, candidates)
        {
            Ok(SamplingOutcome::Token(token)) => SampleOutcome::Sampled(token),
            Ok(SamplingOutcome::AllCandidatesEliminated) => SampleOutcome::AllCandidatesEliminated,
            Ok(SamplingOutcome::GrammarRejectedModelOutput(message)) => {
                SampleOutcome::GrammarRejected(message)
            }
            Err(sampling_error) => SampleOutcome::Failed(format!("{sampling_error:#}")),
        }
    }
}
