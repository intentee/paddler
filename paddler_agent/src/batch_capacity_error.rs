#[derive(Debug, thiserror::Error)]
pub enum BatchCapacityError {
    #[error(
        "a batch of {n_batch} tokens cannot serve {slot_count} slots, because llama.cpp derives the maximum number of logits positions from the batch size"
    )]
    BatchSmallerThanSlotCount { n_batch: usize, slot_count: i32 },
    #[error(
        "a batch of {n_batch} tokens exceeds the context size of {context_size}, and llama.cpp clamps the batch to the context size under causal attention"
    )]
    BatchLargerThanContextSize { n_batch: usize, context_size: u32 },
}
