use crate::batch_capacity_error::BatchCapacityError;

pub struct BatchCapacity {
    pub context_size: u32,
    pub n_batch: usize,
    pub slot_count: i32,
}

impl BatchCapacity {
    /// # Errors
    /// Returns [`BatchCapacityError::BatchSmallerThanSlotCount`] when the batch cannot hold one
    /// logits position per slot, and [`BatchCapacityError::BatchLargerThanContextSize`] when the
    /// batch is larger than llama.cpp will allow it to be.
    pub fn validate(&self) -> Result<(), BatchCapacityError> {
        let slot_count_as_tokens = usize::try_from(self.slot_count).unwrap_or(usize::MAX);

        if slot_count_as_tokens > self.n_batch {
            return Err(BatchCapacityError::BatchSmallerThanSlotCount {
                n_batch: self.n_batch,
                slot_count: self.slot_count,
            });
        }

        if self.n_batch > usize::try_from(self.context_size).unwrap_or(usize::MAX) {
            return Err(BatchCapacityError::BatchLargerThanContextSize {
                n_batch: self.n_batch,
                context_size: self.context_size,
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::BatchCapacity;
    use crate::batch_capacity_error::BatchCapacityError;

    #[test]
    fn a_batch_that_fits_the_context_and_the_slots_is_accepted() {
        assert!(
            BatchCapacity {
                context_size: 8192,
                n_batch: 2048,
                slot_count: 4,
            }
            .validate()
            .is_ok()
        );
    }

    #[test]
    fn a_batch_matching_both_bounds_exactly_is_accepted() {
        assert!(
            BatchCapacity {
                context_size: 4,
                n_batch: 4,
                slot_count: 4,
            }
            .validate()
            .is_ok()
        );
    }

    #[test]
    fn a_batch_smaller_than_the_slot_count_is_rejected() {
        assert!(matches!(
            BatchCapacity {
                context_size: 8192,
                n_batch: 2,
                slot_count: 4,
            }
            .validate(),
            Err(BatchCapacityError::BatchSmallerThanSlotCount {
                n_batch: 2,
                slot_count: 4,
            })
        ));
    }

    #[test]
    fn a_batch_larger_than_the_context_is_rejected() {
        assert!(matches!(
            BatchCapacity {
                context_size: 256,
                n_batch: 512,
                slot_count: 2,
            }
            .validate(),
            Err(BatchCapacityError::BatchLargerThanContextSize {
                n_batch: 512,
                context_size: 256,
            })
        ));
    }

    #[test]
    fn a_negative_slot_count_cannot_exceed_the_batch() {
        assert!(
            BatchCapacity {
                context_size: 8192,
                n_batch: 0,
                slot_count: -1,
            }
            .validate()
            .is_err()
        );
    }
}
