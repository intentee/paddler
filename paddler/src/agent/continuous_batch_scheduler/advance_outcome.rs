use llama_cpp_bindings::SampledToken;
use paddler_types::generated_token_result::GeneratedTokenResult;

pub enum AdvanceOutcome {
    SampledAndStored(SampledToken),
    Completed(GeneratedTokenResult),
    ChannelDropped,
}

#[cfg(test)]
mod tests {
    use paddler_types::generated_token_result::GeneratedTokenResult;
    use paddler_types::generation_summary::GenerationSummary;

    use super::AdvanceOutcome;

    #[test]
    fn completed_carries_event_through_into_inner() {
        let outcome = AdvanceOutcome::Completed(GeneratedTokenResult::Done(
            GenerationSummary::default(),
        ));

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
