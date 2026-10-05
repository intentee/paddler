use anyhow::Result;
use llama_cpp_bindings::SampledToken;
use llama_cpp_bindings::sampled_token_classifier::IngestOutcome;
use llama_cpp_bindings::sampled_token_classifier::SampledTokenSection;
use llama_cpp_bindings::token::LlamaToken;

use crate::continuous_batch_active_request::ContinuousBatchActiveRequest;
use crate::continuous_batch_scheduler::classified_token::ClassifiedToken;

pub fn run(
    request: &mut ContinuousBatchActiveRequest,
    raw_token: LlamaToken,
) -> Result<Vec<ClassifiedToken>> {
    let section_before_ingest = request.token_classifier.current_section();
    let outcomes = request.token_classifier.ingest(raw_token)?;

    Ok(classify_ingest_outcomes(outcomes, section_before_ingest))
}

fn classify_ingest_outcomes(
    outcomes: Vec<IngestOutcome>,
    section_before: SampledTokenSection,
) -> Vec<ClassifiedToken> {
    let mut previous_section = section_before;
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

const fn section_of(token: SampledToken) -> SampledTokenSection {
    match token {
        SampledToken::Reasoning(_) => SampledTokenSection::Reasoning,
        SampledToken::Content(_) => SampledTokenSection::Content,
        SampledToken::ToolCall(_) => SampledTokenSection::ToolCall,
        SampledToken::Undeterminable(_) => SampledTokenSection::Pending,
    }
}

#[cfg(test)]
mod tests {
    use llama_cpp_bindings::SampledToken;
    use llama_cpp_bindings::sampled_token_classifier::IngestOutcome;
    use llama_cpp_bindings::sampled_token_classifier::SampledTokenSection;
    use llama_cpp_bindings::token::LlamaToken;

    use super::classify_ingest_outcomes;

    fn outcome(sampled: SampledToken) -> IngestOutcome {
        IngestOutcome {
            sampled_token: sampled,
            visible_piece: String::new(),
            raw_piece: String::new(),
        }
    }

    #[test]
    fn content_after_content_stays_outside_tool_call() {
        let classified = classify_ingest_outcomes(
            vec![outcome(SampledToken::Content(LlamaToken::new(1)))],
            SampledTokenSection::Content,
        );

        assert_eq!(classified.len(), 1);
        assert!(!classified[0].was_in_tool_call);
        assert!(!classified[0].is_in_tool_call);
    }

    #[test]
    fn content_to_tool_call_marks_entry_transition() {
        let classified = classify_ingest_outcomes(
            vec![outcome(SampledToken::ToolCall(LlamaToken::new(2)))],
            SampledTokenSection::Content,
        );

        assert_eq!(classified.len(), 1);
        assert!(!classified[0].was_in_tool_call);
        assert!(classified[0].is_in_tool_call);
    }

    #[test]
    fn tool_call_to_tool_call_stays_inside() {
        let classified = classify_ingest_outcomes(
            vec![outcome(SampledToken::ToolCall(LlamaToken::new(3)))],
            SampledTokenSection::ToolCall,
        );

        assert_eq!(classified.len(), 1);
        assert!(classified[0].was_in_tool_call);
        assert!(classified[0].is_in_tool_call);
    }

    #[test]
    fn tool_call_to_content_marks_exit_transition() {
        let classified = classify_ingest_outcomes(
            vec![outcome(SampledToken::Content(LlamaToken::new(4)))],
            SampledTokenSection::ToolCall,
        );

        assert_eq!(classified.len(), 1);
        assert!(classified[0].was_in_tool_call);
        assert!(!classified[0].is_in_tool_call);
    }

    #[test]
    fn reasoning_after_content_stays_outside_tool_call() {
        let classified = classify_ingest_outcomes(
            vec![outcome(SampledToken::Reasoning(LlamaToken::new(5)))],
            SampledTokenSection::Content,
        );

        assert_eq!(classified.len(), 1);
        assert!(!classified[0].was_in_tool_call);
        assert!(!classified[0].is_in_tool_call);
    }

    #[test]
    fn undeterminable_after_content_maps_to_pending() {
        let classified = classify_ingest_outcomes(
            vec![outcome(SampledToken::Undeterminable(LlamaToken::new(6)))],
            SampledTokenSection::Content,
        );

        assert_eq!(classified.len(), 1);
        assert!(!classified[0].was_in_tool_call);
        assert!(!classified[0].is_in_tool_call);
    }

    #[test]
    fn previous_section_carries_forward_across_multi_outcome_vec() {
        let outcomes = vec![
            outcome(SampledToken::ToolCall(LlamaToken::new(7))),
            outcome(SampledToken::ToolCall(LlamaToken::new(8))),
            outcome(SampledToken::Content(LlamaToken::new(9))),
        ];

        let classified = classify_ingest_outcomes(outcomes, SampledTokenSection::Content);

        assert_eq!(classified.len(), 3);

        assert!(!classified[0].was_in_tool_call);
        assert!(classified[0].is_in_tool_call);

        assert!(classified[1].was_in_tool_call);
        assert!(classified[1].is_in_tool_call);

        assert!(classified[2].was_in_tool_call);
        assert!(!classified[2].is_in_tool_call);
    }
}
