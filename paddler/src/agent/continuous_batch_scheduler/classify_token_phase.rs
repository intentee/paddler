use llama_cpp_bindings::SampledToken;
use llama_cpp_bindings::sampled_token_classifier::SampledTokenSection;
use llama_cpp_bindings::token::LlamaToken;

use crate::agent::continuous_batch_active_request::ContinuousBatchActiveRequest;
use crate::agent::continuous_batch_scheduler::classified_token::ClassifiedToken;

pub struct ClassifyTokenPhase;

impl ClassifyTokenPhase {
    pub fn run(
        self,
        request: &mut ContinuousBatchActiveRequest,
        raw_token: LlamaToken,
    ) -> Vec<ClassifiedToken> {
        let section_before_ingest = request.token_classifier.current_section();
        let outcomes = request.token_classifier.ingest(raw_token);

        let mut previous_section = section_before_ingest;
        outcomes
            .into_iter()
            .map(|outcome| {
                let section = section_of(outcome.sampled_token);
                let classified = ClassifiedToken {
                    sampled_token: outcome.sampled_token,
                    was_in_tool_call: previous_section == SampledTokenSection::ToolCall,
                    is_in_tool_call: section == SampledTokenSection::ToolCall,
                    visible_piece: outcome.visible_piece,
                    raw_piece: outcome.raw_piece,
                };
                previous_section = section;
                classified
            })
            .collect()
    }
}

const fn section_of(token: SampledToken) -> SampledTokenSection {
    match token {
        SampledToken::Reasoning(_) => SampledTokenSection::Reasoning,
        SampledToken::Content(_) => SampledTokenSection::Content,
        SampledToken::ToolCall(_) => SampledTokenSection::ToolCall,
        SampledToken::Undeterminable(_) => SampledTokenSection::Pending,
    }
}
