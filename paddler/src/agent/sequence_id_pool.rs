pub struct SequenceIdPool {
    available_ids: Vec<i32>,
}

impl SequenceIdPool {
    #[must_use]
    pub fn new(max_sequences: i32) -> Self {
        let available_ids = (0..max_sequences).rev().collect();

        Self { available_ids }
    }

    pub fn acquire(&mut self) -> Option<i32> {
        self.available_ids.pop()
    }

    pub fn release(&mut self, sequence_id: i32) {
        self.available_ids.push(sequence_id);
    }

    #[must_use]
    pub const fn available_count(&self) -> usize {
        self.available_ids.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acquire_returns_sequential_ids() {
        let mut pool = SequenceIdPool::new(4);

        assert_eq!(pool.acquire(), Some(0));
        assert_eq!(pool.acquire(), Some(1));
        assert_eq!(pool.acquire(), Some(2));
        assert_eq!(pool.acquire(), Some(3));
    }

    #[test]
    fn release_makes_id_available_again() {
        let mut pool = SequenceIdPool::new(2);

        let first_id = pool.acquire();
        assert_eq!(first_id, Some(0));

        pool.release(0);

        let reacquired_id = pool.acquire();
        assert_eq!(reacquired_id, Some(0));
    }

    #[test]
    fn acquire_returns_none_when_exhausted() {
        let mut pool = SequenceIdPool::new(2);

        assert_eq!(pool.acquire(), Some(0));
        assert_eq!(pool.acquire(), Some(1));
        assert_eq!(pool.acquire(), None);
    }

    #[test]
    fn available_count_tracks_pool_size() {
        let mut pool = SequenceIdPool::new(3);

        assert_eq!(pool.available_count(), 3);

        pool.acquire();
        assert_eq!(pool.available_count(), 2);

        pool.acquire();
        assert_eq!(pool.available_count(), 1);

        pool.release(0);
        assert_eq!(pool.available_count(), 2);
    }
}
