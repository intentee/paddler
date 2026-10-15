use paddler_agent_runtime::sequence_id::SequenceId;

pub fn sequence_ordered_insertion_index(
    active_sequence_ids: impl IntoIterator<Item = SequenceId>,
    sequence_id: SequenceId,
) -> usize {
    active_sequence_ids
        .into_iter()
        .take_while(|active_sequence_id| *active_sequence_id < sequence_id)
        .count()
}

#[cfg(test)]
mod tests {
    use paddler_agent_runtime::sequence_id::SequenceId;

    use super::sequence_ordered_insertion_index;

    fn active_sequence_ids() -> [SequenceId; 3] {
        [SequenceId(1), SequenceId(4), SequenceId(6)]
    }

    #[test]
    fn places_a_sequence_before_every_higher_sequence_id() {
        assert_eq!(
            sequence_ordered_insertion_index(active_sequence_ids(), SequenceId(5)),
            2
        );
    }

    #[test]
    fn places_the_lowest_sequence_id_first() {
        assert_eq!(
            sequence_ordered_insertion_index(active_sequence_ids(), SequenceId(0)),
            0
        );
    }

    #[test]
    fn places_the_highest_sequence_id_last() {
        assert_eq!(
            sequence_ordered_insertion_index(active_sequence_ids(), SequenceId(7)),
            3
        );
    }
}
