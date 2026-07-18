use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone)]
pub struct SequenceIdPool {
    available_ids: Rc<RefCell<Vec<i32>>>,
}

impl SequenceIdPool {
    #[must_use]
    pub fn new(max_sequences: i32) -> Self {
        Self {
            available_ids: Rc::new(RefCell::new((0..max_sequences).rev().collect())),
        }
    }

    #[must_use]
    pub fn acquire(&self) -> Option<i32> {
        self.available_ids.borrow_mut().pop()
    }

    pub fn release(&self, sequence_id: i32) {
        self.available_ids.borrow_mut().push(sequence_id);
    }

    #[must_use]
    pub fn available_count(&self) -> usize {
        self.available_ids.borrow().len()
    }
}

#[cfg(test)]
mod tests {
    use super::SequenceIdPool;

    #[test]
    fn acquire_returns_sequential_ids() {
        let pool = SequenceIdPool::new(4);

        assert_eq!(pool.acquire(), Some(0));
        assert_eq!(pool.acquire(), Some(1));
        assert_eq!(pool.acquire(), Some(2));
        assert_eq!(pool.acquire(), Some(3));
    }

    #[test]
    fn release_makes_id_available_again() {
        let pool = SequenceIdPool::new(2);

        let first_id = pool.acquire();
        assert_eq!(first_id, Some(0));

        pool.release(0);

        let reacquired_id = pool.acquire();
        assert_eq!(reacquired_id, Some(0));
    }

    #[test]
    fn acquire_returns_none_when_exhausted() {
        let pool = SequenceIdPool::new(2);

        assert_eq!(pool.acquire(), Some(0));
        assert_eq!(pool.acquire(), Some(1));
        assert_eq!(pool.acquire(), None);
    }

    #[test]
    fn available_count_tracks_pool_size() {
        let pool = SequenceIdPool::new(3);

        assert_eq!(pool.available_count(), 3);

        assert!(pool.acquire().is_some());
        assert_eq!(pool.available_count(), 2);

        assert!(pool.acquire().is_some());
        assert_eq!(pool.available_count(), 1);

        pool.release(0);
        assert_eq!(pool.available_count(), 2);
    }

    #[test]
    fn cloned_handles_share_the_same_available_ids() {
        let pool = SequenceIdPool::new(2);
        let cloned_pool = pool.clone();

        assert_eq!(pool.acquire(), Some(0));
        assert_eq!(cloned_pool.available_count(), 1);

        cloned_pool.release(0);
        assert_eq!(pool.available_count(), 2);
    }
}
