pub fn sequence_ordered_insertion_index(
    active_sequence_ids: impl IntoIterator<Item = i32>,
    sequence_id: i32,
) -> usize {
    active_sequence_ids
        .into_iter()
        .take_while(|active_sequence_id| *active_sequence_id < sequence_id)
        .count()
}

#[cfg(test)]
mod tests {
    use super::sequence_ordered_insertion_index;

    #[test]
    fn places_a_sequence_before_every_higher_sequence_id() {
        assert_eq!(sequence_ordered_insertion_index([1, 4, 6], 5), 2);
    }

    #[test]
    fn places_the_lowest_sequence_id_first() {
        assert_eq!(sequence_ordered_insertion_index([1, 4, 6], 0), 0);
    }

    #[test]
    fn places_the_highest_sequence_id_last() {
        assert_eq!(sequence_ordered_insertion_index([1, 4, 6], 7), 3);
    }
}
