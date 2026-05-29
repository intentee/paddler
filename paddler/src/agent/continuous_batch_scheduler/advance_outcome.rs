use crate::generated_token_result::GeneratedTokenResult;
use llama_cpp_bindings::SampledToken;

pub enum AdvanceOutcome {
    SampledAndStored(SampledToken),
    Completed(GeneratedTokenResult),
    ChannelDropped,
}

#[cfg(test)]
mod tests {
    use crate::generated_token_result::GeneratedTokenResult;
    use crate::generation_summary::GenerationSummary;

    use super::AdvanceOutcome;

    #[test]
    fn completed_carries_event_through_into_inner() {
        let outcome =
            AdvanceOutcome::Completed(GeneratedTokenResult::Done(GenerationSummary::default()));

        assert!(matches!(
            outcome,
            AdvanceOutcome::Completed(GeneratedTokenResult::Done(_))
        ));
    }

    #[test]
    fn channel_dropped_is_distinct_variant() {
        let outcome = AdvanceOutcome::ChannelDropped;

        assert!(matches!(outcome, AdvanceOutcome::ChannelDropped));
    }
}
