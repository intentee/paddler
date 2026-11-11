use std::sync::Arc;

use crate::buffered_request_counter::BufferedRequestCounter;

pub struct BufferedRequestCountGuard {
    buffered_requests_counter: Arc<BufferedRequestCounter>,
}

impl BufferedRequestCountGuard {
    pub fn try_admit(buffered_requests_counter: &Arc<BufferedRequestCounter>) -> Option<Self> {
        buffered_requests_counter.try_increment().then(|| Self {
            buffered_requests_counter: buffered_requests_counter.clone(),
        })
    }
}

impl Drop for BufferedRequestCountGuard {
    fn drop(&mut self) {
        self.buffered_requests_counter.decrement();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use tokio::sync::watch;

    use super::BufferedRequestCountGuard;
    use crate::buffered_request_counter::BufferedRequestCounter;

    fn counter_with_capacity(max_buffered_requests: u64) -> Arc<BufferedRequestCounter> {
        let (update_tx, _initial_rx) = watch::channel(());

        Arc::new(BufferedRequestCounter::new(
            update_tx,
            max_buffered_requests,
        ))
    }

    #[test]
    fn admits_requests_while_below_capacity() {
        let counter = counter_with_capacity(2);
        let _first_admission = BufferedRequestCountGuard::try_admit(&counter);
        let _second_admission = BufferedRequestCountGuard::try_admit(&counter);

        assert_eq!(counter.get(), 2);
    }

    #[test]
    fn refuses_a_request_at_capacity() {
        let counter = counter_with_capacity(1);
        let _admission = BufferedRequestCountGuard::try_admit(&counter);

        assert!(BufferedRequestCountGuard::try_admit(&counter).is_none());
        assert_eq!(counter.get(), 1);
    }

    #[test]
    fn releasing_an_admission_frees_its_place() {
        let counter = counter_with_capacity(1);

        drop(BufferedRequestCountGuard::try_admit(&counter));

        assert_eq!(counter.get(), 0);
        assert!(BufferedRequestCountGuard::try_admit(&counter).is_some());
    }
}
