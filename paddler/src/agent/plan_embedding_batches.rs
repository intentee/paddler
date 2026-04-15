use std::ops::Range;

#[must_use]
pub fn plan_embedding_batches(
    token_counts: &[usize],
    batch_n_tokens: usize,
    max_sequences_per_batch: i32,
) -> Vec<Range<usize>> {
    let mut batches = Vec::new();
    let mut start = 0usize;
    let mut current_tokens: usize = 0;
    let mut current_sequences: i32 = 0;

    for (index, &token_count) in token_counts.iter().enumerate() {
        let would_exceed_tokens = current_tokens + token_count > batch_n_tokens;
        let would_exceed_sequences = current_sequences >= max_sequences_per_batch;

        if (would_exceed_tokens || would_exceed_sequences) && current_sequences > 0 {
            batches.push(start..index);
            start = index;
            current_tokens = 0;
            current_sequences = 0;
        }

        current_tokens += token_count;
        current_sequences += 1;
    }

    if current_sequences > 0 {
        batches.push(start..token_counts.len());
    }

    batches
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_yields_no_batches() {
        let batches = plan_embedding_batches(&[], 512, 4);

        assert!(batches.is_empty());
    }

    #[test]
    fn inputs_under_both_caps_fit_in_single_batch() {
        let batches = plan_embedding_batches(&[10, 20, 30], 512, 4);

        assert_eq!(batches, vec![0..3]);
    }

    #[test]
    fn batch_flushes_when_sequence_cap_reached() {
        let batches = plan_embedding_batches(&[10, 10, 10, 10, 10], 512, 2);

        assert_eq!(batches, vec![0..2, 2..4, 4..5]);
    }

    #[test]
    fn batch_flushes_when_token_cap_would_be_exceeded() {
        let batches = plan_embedding_batches(&[300, 300, 300], 512, 16);

        assert_eq!(batches, vec![0..1, 1..2, 2..3]);
    }

    #[test]
    fn oversized_single_input_gets_its_own_batch() {
        let batches = plan_embedding_batches(&[50, 1000, 50], 512, 16);

        assert_eq!(batches, vec![0..1, 1..2, 2..3]);
    }

    #[test]
    fn batch_exactly_at_sequence_cap_starts_new_batch_on_next_input() {
        let batches = plan_embedding_batches(&[10, 10, 10, 10], 512, 4);

        assert_eq!(batches, vec![0..4]);
    }

    #[test]
    fn input_count_three_times_slot_cap_splits_into_three_batches() {
        let batches = plan_embedding_batches(&[10; 12], 512, 4);

        assert_eq!(batches, vec![0..4, 4..8, 8..12]);
    }
}
