use std::sync::Arc;
use std::sync::atomic::AtomicI32;

use tokio::sync::Notify;

use crate::atomic_value::AtomicValue;
use crate::balancer::buffered_request_count_guard::BufferedRequestCountGuard;

pub struct BufferedRequestCounter {
    count: Arc<AtomicValue<AtomicI32>>,
    pub update_notifier: Arc<Notify>,
}

impl BufferedRequestCounter {
    pub fn new(update_notifier: Arc<Notify>) -> Self {
        Self {
            count: Arc::new(AtomicValue::<AtomicI32>::new(0)),
            update_notifier,
        }
    }

    pub fn decrement(&self) {
        self.count.decrement();
        self.update_notifier.notify_waiters();
    }

    pub fn get(&self) -> i32 {
        self.count.get()
    }

    pub fn increment(&self) {
        self.count.increment();
        self.update_notifier.notify_waiters();
    }

    pub fn increment_with_guard(self: &Arc<Self>) -> BufferedRequestCountGuard {
        self.increment();

        BufferedRequestCountGuard::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_counter() -> BufferedRequestCounter {
        BufferedRequestCounter::new(Arc::new(Notify::new()))
    }

    #[test]
    fn starts_at_zero() {
        let counter = make_counter();

        assert_eq!(counter.get(), 0);
    }

    #[test]
    fn increment_increases_count() {
        let counter = make_counter();

        counter.increment();
        counter.increment();

        assert_eq!(counter.get(), 2);
    }

    #[test]
    fn decrement_decreases_count() {
        let counter = make_counter();

        counter.increment();
        counter.increment();
        counter.decrement();

        assert_eq!(counter.get(), 1);
    }

    #[test]
    fn increment_with_guard_decrements_on_drop() {
        let counter = Arc::new(make_counter());

        let guard = counter.increment_with_guard();

        assert_eq!(counter.get(), 1);

        drop(guard);

        assert_eq!(counter.get(), 0);
    }
}
