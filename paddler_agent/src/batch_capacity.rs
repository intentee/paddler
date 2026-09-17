use crate::batch_capacity_error::BatchCapacityError;

pub struct BatchCapacity {
    pub n_batch: usize,
    pub slot_count: i32,
}

impl BatchCapacity {
    /// # Errors
    /// Returns [`BatchCapacityError::BatchSmallerThanSlotCount`] when the batch cannot hold one
    /// logits position per slot.
    pub fn validate(&self) -> Result<(), BatchCapacityError> {
        let slot_count_as_tokens = usize::try_from(self.slot_count).unwrap_or(usize::MAX);

        if slot_count_as_tokens > self.n_batch {
            return Err(BatchCapacityError::BatchSmallerThanSlotCount {
                n_batch: self.n_batch,
                slot_count: self.slot_count,
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
    fn a_batch_larger_than_the_slot_count_is_accepted() {
        assert!(
            BatchCapacity {
                n_batch: 2048,
                slot_count: 4,
            }
            .validate()
            .is_ok()
        );
    }

    #[test]
    fn a_batch_matching_the_slot_count_is_accepted() {
        assert!(
            BatchCapacity {
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
    fn a_negative_slot_count_cannot_exceed_the_batch() {
        assert!(
            BatchCapacity {
                n_batch: 0,
                slot_count: -1,
            }
            .validate()
            .is_err()
        );
    }
}
