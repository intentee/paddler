use llama_cpp_bindings::token::LlamaToken;

use crate::agent::continuous_batch_active_request::ContinuousBatchActiveRequest;
use crate::agent::continuous_batch_scheduler::classified_token::ClassifiedToken;

pub struct ClassifyTokenPhase;

impl ClassifyTokenPhase {
    pub fn run(
        self,
        request: &mut ContinuousBatchActiveRequest,
        raw_token: LlamaToken,
    ) -> ClassifiedToken {
        let was_in_tool_call = request.token_classifier.is_in_tool_call();
        let sampled_token = request.token_classifier.ingest(raw_token);
        let is_in_tool_call = request.token_classifier.is_in_tool_call();

        ClassifiedToken {
            sampled_token,
            was_in_tool_call,
            is_in_tool_call,
        }
    }
}
