use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use tokio::sync::watch;

use paddler_messaging::atomic_value::AtomicValue;

use crate::buffered_request_count_guard::BufferedRequestCountGuard;

pub struct BufferedRequestCounter {
    count: AtomicValue<AtomicU64>,
    max_buffered_requests: u64,
    update_tx: watch::Sender<()>,
}

impl BufferedRequestCounter {
    pub const fn new(update_tx: watch::Sender<()>, max_buffered_requests: u64) -> Self {
        Self {
            count: AtomicValue::<AtomicU64>::new(0),
            max_buffered_requests,
            update_tx,
        }
    }

    pub fn decrement(&self) {
        self.count.decrement();
        self.update_tx.send_replace(());
    }

    pub fn get(&self) -> u64 {
        self.count.get()
    }

    pub fn try_admit(self: &Arc<Self>) -> Option<BufferedRequestCountGuard> {
        self.count
            .increment_below(self.max_buffered_requests)
            .then(|| {
                self.update_tx.send_replace(());

                BufferedRequestCountGuard::new(self.clone())
            })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use tokio::sync::watch;

    use super::BufferedRequestCounter;

    fn counter_with_capacity(max_buffered_requests: u64) -> Arc<BufferedRequestCounter> {
        let (update_tx, _initial_rx) = watch::channel(());

        Arc::new(BufferedRequestCounter::new(
            update_tx,
            max_buffered_requests,
        ))
    }

    #[test]
    fn starts_at_zero() {
        assert_eq!(counter_with_capacity(1).get(), 0);
    }

    #[test]
    fn admits_requests_while_below_capacity() {
        let counter = counter_with_capacity(2);
        let _first_admission = counter.try_admit();
        let _second_admission = counter.try_admit();

        assert_eq!(counter.get(), 2);
    }

    #[test]
    fn refuses_a_request_at_capacity() {
        let counter = counter_with_capacity(1);
        let _admission = counter.try_admit();

        assert!(counter.try_admit().is_none());
        assert_eq!(counter.get(), 1);
    }

    #[test]
    fn releasing_an_admission_frees_its_place() {
        let counter = counter_with_capacity(1);

        drop(counter.try_admit());

        assert_eq!(counter.get(), 0);
        assert!(counter.try_admit().is_some());
    }
}
