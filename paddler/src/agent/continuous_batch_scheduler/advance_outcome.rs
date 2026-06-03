use crate::generated_token_result::GeneratedTokenResult;
use llama_cpp_bindings::SampledToken;

pub enum AdvanceOutcome {
    SampledAndStored(SampledToken),
    Completed(GeneratedTokenResult),
    ChannelDropped,
}

#[cfg(test)]
mod tests {
    use std::mem::discriminant;

    use llama_cpp_bindings::SampledToken;
    use llama_cpp_bindings::token::LlamaToken;

    use crate::generated_token_result::GeneratedTokenResult;
    use crate::generation_summary::GenerationSummary;

    use super::AdvanceOutcome;

    #[test]
    fn sampled_and_stored_is_distinct_from_the_other_variants() {
        let sampled_and_stored =
            AdvanceOutcome::SampledAndStored(SampledToken::Content(LlamaToken::new(7)));

        assert_eq!(
            discriminant(&sampled_and_stored),
            discriminant(&AdvanceOutcome::SampledAndStored(SampledToken::Reasoning(
                LlamaToken::new(0)
            )))
        );
        assert_ne!(
            discriminant(&sampled_and_stored),
            discriminant(&AdvanceOutcome::Completed(GeneratedTokenResult::Done(
                GenerationSummary::default()
            )))
        );
        assert_ne!(
            discriminant(&sampled_and_stored),
            discriminant(&AdvanceOutcome::ChannelDropped)
        );
    }

    #[test]
    fn completed_is_distinct_from_the_other_variants() {
        let completed =
            AdvanceOutcome::Completed(GeneratedTokenResult::Done(GenerationSummary::default()));

        assert_eq!(
            discriminant(&completed),
            discriminant(&AdvanceOutcome::Completed(
                GeneratedTokenResult::ContentToken("next".to_owned())
            ))
        );
        assert_ne!(
            discriminant(&completed),
            discriminant(&AdvanceOutcome::ChannelDropped)
        );
    }

    #[test]
    fn channel_dropped_is_distinct_from_the_other_variants() {
        let channel_dropped = AdvanceOutcome::ChannelDropped;

        assert_eq!(
            discriminant(&channel_dropped),
            discriminant(&AdvanceOutcome::ChannelDropped)
        );
        assert_ne!(
            discriminant(&channel_dropped),
            discriminant(&AdvanceOutcome::SampledAndStored(SampledToken::ToolCall(
                LlamaToken::new(0)
            )))
        );
    }
}
