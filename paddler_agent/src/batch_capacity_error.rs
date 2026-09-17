#[derive(Debug, thiserror::Error)]
pub enum BatchCapacityError {
    #[error(
        "a batch of {n_batch} tokens cannot serve {slot_count} slots, because llama.cpp derives the maximum number of logits positions from the batch size"
    )]
    BatchSmallerThanSlotCount { n_batch: usize, slot_count: i32 },
}
