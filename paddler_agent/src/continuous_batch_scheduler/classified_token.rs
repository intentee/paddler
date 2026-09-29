use llama_cpp_bindings::SampledToken;
use llama_cpp_bindings::SampledTokenSection;
use llama_cpp_bindings::ingest_outcome::IngestOutcome;
use llama_cpp_bindings::token_piece::TokenPiece;

const fn section_of(token: SampledToken) -> SampledTokenSection {
    match token {
        SampledToken::Reasoning(_) => SampledTokenSection::Reasoning,
        SampledToken::Content(_) => SampledTokenSection::Content,
        SampledToken::ToolCall(_) => SampledTokenSection::ToolCall,
        SampledToken::Undeterminable(_) => SampledTokenSection::Pending,
    }
}

pub struct ClassifiedToken {
    pub sampled_token: SampledToken,
    pub piece: TokenPiece,
    pub was_in_tool_call: bool,
    pub is_in_tool_call: bool,
}

impl ClassifiedToken {
    #[must_use]
    pub fn classify(
        IngestOutcome {
            sampled_token,
            piece,
        }: IngestOutcome,
        last_outcome_section: &mut SampledTokenSection,
    ) -> Self {
        let section = section_of(sampled_token);
        let classified = Self {
            sampled_token,
            piece,
            was_in_tool_call: *last_outcome_section == SampledTokenSection::ToolCall,
            is_in_tool_call: section == SampledTokenSection::ToolCall,
        };

        *last_outcome_section = section;

        classified
    }
}

#[cfg(test)]
mod tests {
    use llama_cpp_bindings::SampledToken;
    use llama_cpp_bindings::SampledTokenSection;
    use llama_cpp_bindings::ingest_outcome::IngestOutcome;
    use llama_cpp_bindings::token::LlamaToken;
    use llama_cpp_bindings::token_piece::TokenPiece;

    use super::ClassifiedToken;

    fn classify(
        sampled_token: SampledToken,
        last_outcome_section: &mut SampledTokenSection,
    ) -> ClassifiedToken {
        ClassifiedToken::classify(
            IngestOutcome {
                sampled_token,
                piece: TokenPiece::Visible(String::new()),
            },
            last_outcome_section,
        )
    }

    #[test]
    fn content_after_content_stays_outside_tool_call() {
        let classified = classify(
            SampledToken::Content(LlamaToken::new(1)),
            &mut SampledTokenSection::Content,
        );

        assert!(!classified.was_in_tool_call);
        assert!(!classified.is_in_tool_call);
    }

    #[test]
    fn content_to_tool_call_marks_entry_transition() {
        let classified = classify(
            SampledToken::ToolCall(LlamaToken::new(2)),
            &mut SampledTokenSection::Content,
        );

        assert!(!classified.was_in_tool_call);
        assert!(classified.is_in_tool_call);
    }

    #[test]
    fn tool_call_to_tool_call_stays_inside() {
        let classified = classify(
            SampledToken::ToolCall(LlamaToken::new(3)),
            &mut SampledTokenSection::ToolCall,
        );

        assert!(classified.was_in_tool_call);
        assert!(classified.is_in_tool_call);
    }

    #[test]
    fn tool_call_to_content_marks_exit_transition() {
        let classified = classify(
            SampledToken::Content(LlamaToken::new(4)),
            &mut SampledTokenSection::ToolCall,
        );

        assert!(classified.was_in_tool_call);
        assert!(!classified.is_in_tool_call);
    }

    #[test]
    fn reasoning_after_content_stays_outside_tool_call() {
        let classified = classify(
            SampledToken::Reasoning(LlamaToken::new(5)),
            &mut SampledTokenSection::Content,
        );

        assert!(!classified.was_in_tool_call);
        assert!(!classified.is_in_tool_call);
    }

    #[test]
    fn undeterminable_token_moves_the_section_to_pending() {
        let mut last_outcome_section = SampledTokenSection::Content;

        classify(
            SampledToken::Undeterminable(LlamaToken::new(6)),
            &mut last_outcome_section,
        );

        assert_eq!(last_outcome_section, SampledTokenSection::Pending);
    }

    #[test]
    fn exit_transition_is_detected_across_separate_outcomes() {
        let mut last_outcome_section = SampledTokenSection::Content;

        classify(
            SampledToken::ToolCall(LlamaToken::new(10)),
            &mut last_outcome_section,
        );

        let classified = classify(
            SampledToken::Content(LlamaToken::new(11)),
            &mut last_outcome_section,
        );

        assert!(classified.was_in_tool_call);
        assert!(!classified.is_in_tool_call);
    }
}
