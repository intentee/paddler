use crate::sequence_id_pool::SequenceIdPool;

pub struct SequenceIdGuard {
    sequence_id: i32,
    sequence_id_pool: SequenceIdPool,
}

impl SequenceIdGuard {
    #[must_use]
    pub fn acquire(sequence_id_pool: &SequenceIdPool) -> Option<Self> {
        sequence_id_pool.acquire().map(|sequence_id| Self {
            sequence_id,
            sequence_id_pool: sequence_id_pool.clone(),
        })
    }

    #[must_use]
    pub const fn sequence_id(&self) -> i32 {
        self.sequence_id
    }
}

impl Drop for SequenceIdGuard {
    fn drop(&mut self) {
        self.sequence_id_pool.release(self.sequence_id);
    }
}

#[cfg(test)]
mod tests {
    use super::SequenceIdGuard;
    use crate::sequence_id_pool::SequenceIdPool;

    #[test]
    fn acquire_takes_a_sequence_id_from_the_pool() {
        let sequence_id_pool = SequenceIdPool::new(1);

        let guard = SequenceIdGuard::acquire(&sequence_id_pool).unwrap();

        assert_eq!(guard.sequence_id(), 0);
        assert_eq!(sequence_id_pool.available_count(), 0);
    }

    #[test]
    fn acquire_returns_none_when_the_pool_is_exhausted() {
        let sequence_id_pool = SequenceIdPool::new(1);

        let _first_guard = SequenceIdGuard::acquire(&sequence_id_pool).unwrap();

        assert!(SequenceIdGuard::acquire(&sequence_id_pool).is_none());
    }

    #[test]
    fn dropping_an_uncommitted_guard_releases_the_sequence_id() {
        let sequence_id_pool = SequenceIdPool::new(1);

        {
            let _guard = SequenceIdGuard::acquire(&sequence_id_pool).unwrap();

            assert_eq!(sequence_id_pool.available_count(), 0);
        }

        assert_eq!(sequence_id_pool.available_count(), 1);
    }
}
