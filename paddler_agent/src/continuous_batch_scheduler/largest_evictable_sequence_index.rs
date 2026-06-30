use crate::continuous_batch_request_phase::ContinuousBatchRequestPhase;
use crate::continuous_batch_request_state::ContinuousBatchRequestState;

#[must_use]
pub fn largest_evictable_sequence_index(
    request_states: &[&ContinuousBatchRequestState],
) -> Option<usize> {
    let mut largest_sequence_index: Option<usize> = None;
    let mut largest_position: i32 = -1;

    for (index, request_state) in request_states.iter().enumerate() {
        if matches!(request_state.phase, ContinuousBatchRequestPhase::Completed) {
            continue;
        }

        if request_state.current_token_position > largest_position {
            largest_position = request_state.current_token_position;
            largest_sequence_index = Some(index);
        }
    }

    largest_sequence_index
}

#[cfg(test)]
mod tests {
    use llama_cpp_bindings::token::LlamaToken;

    use super::largest_evictable_sequence_index;
    use crate::continuous_batch_request_phase::ContinuousBatchRequestPhase;
    use crate::continuous_batch_request_state::ContinuousBatchRequestState;

    fn request_state(
        phase: ContinuousBatchRequestPhase,
        current_token_position: i32,
    ) -> ContinuousBatchRequestState {
        ContinuousBatchRequestState {
            current_token_position,
            i_batch: None,
            max_tokens: 64,
            pending_sampled_token: None,
            phase,
            prompt_tokens: vec![LlamaToken::new(1)],
            prompt_tokens_ingested: 0,
            sequence_id: 0,
        }
    }

    #[test]
    fn empty_input_yields_no_index() {
        assert_eq!(largest_evictable_sequence_index(&[]), None);
    }

    #[test]
    fn all_completed_requests_yield_no_index() {
        let first = request_state(ContinuousBatchRequestPhase::Completed, 10);
        let second = request_state(ContinuousBatchRequestPhase::Completed, 20);

        assert_eq!(largest_evictable_sequence_index(&[&first, &second]), None);
    }

    #[test]
    fn selects_the_request_with_the_largest_token_position() {
        let first = request_state(ContinuousBatchRequestPhase::Generating, 2);
        let second = request_state(ContinuousBatchRequestPhase::Generating, 5);
        let third = request_state(ContinuousBatchRequestPhase::Generating, 3);

        assert_eq!(
            largest_evictable_sequence_index(&[&first, &second, &third]),
            Some(1)
        );
    }

    #[test]
    fn completed_requests_are_skipped_during_selection() {
        let first = request_state(ContinuousBatchRequestPhase::Completed, 100);
        let second = request_state(ContinuousBatchRequestPhase::Generating, 4);

        assert_eq!(
            largest_evictable_sequence_index(&[&first, &second]),
            Some(1)
        );
    }

    #[test]
    fn ties_keep_the_first_seen_request() {
        let first = request_state(ContinuousBatchRequestPhase::Generating, 7);
        let second = request_state(ContinuousBatchRequestPhase::Generating, 7);

        assert_eq!(
            largest_evictable_sequence_index(&[&first, &second]),
            Some(0)
        );
    }

    #[test]
    fn a_request_at_position_zero_is_still_evictable() {
        let only = request_state(ContinuousBatchRequestPhase::Generating, 0);

        assert_eq!(largest_evictable_sequence_index(&[&only]), Some(0));
    }
}
