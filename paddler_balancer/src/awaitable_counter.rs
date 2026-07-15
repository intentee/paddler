use std::sync::Arc;
use std::sync::atomic::AtomicI32;

use tokio::sync::Notify;
use tokio::sync::watch;

use paddler_messaging::atomic_value::AtomicValue;

use crate::awaitable_counter_guard::AwaitableCounterGuard;

pub struct AwaitableCounter {
    count: Arc<AtomicValue<AtomicI32>>,
    update_tx: watch::Sender<()>,
    zero_notify: Notify,
}

impl AwaitableCounter {
    pub fn decrement(&self) {
        self.count.decrement();
        self.update_tx.send_replace(());
        self.zero_notify.notify_one();
    }

    pub fn get(&self) -> i32 {
        self.count.get()
    }

    pub fn increment(&self) {
        self.count.increment();
        self.update_tx.send_replace(());
    }

    pub fn increment_with_guard(self: &Arc<Self>) -> AwaitableCounterGuard {
        self.increment();

        AwaitableCounterGuard::new(self.clone())
    }

    pub fn subscribe(&self) -> watch::Receiver<()> {
        self.update_tx.subscribe()
    }

    pub async fn wait_for_zero(&self) {
        loop {
            if self.get() <= 0 {
                return;
            }

            self.zero_notify.notified().await;
        }
    }
}

impl Default for AwaitableCounter {
    fn default() -> Self {
        let (update_tx, _initial_rx) = watch::channel(());

        Self {
            count: Arc::new(AtomicValue::<AtomicI32>::new(0)),
            update_tx,
            zero_notify: Notify::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::time::timeout;

    use super::*;

    #[test]
    fn starts_at_zero() {
        let counter = AwaitableCounter::default();

        assert_eq!(counter.get(), 0);
    }

    #[test]
    fn increment_increases_count() {
        let counter = AwaitableCounter::default();

        counter.increment();
        counter.increment();

        assert_eq!(counter.get(), 2);
    }

    #[test]
    fn decrement_decreases_count() {
        let counter = AwaitableCounter::default();

        counter.increment();
        counter.increment();
        counter.decrement();

        assert_eq!(counter.get(), 1);
    }

    #[test]
    fn increment_with_guard_decrements_on_drop() {
        let counter = Arc::new(AwaitableCounter::default());

        let guard = counter.increment_with_guard();

        assert_eq!(counter.get(), 1);

        drop(guard);

        assert_eq!(counter.get(), 0);
    }

    #[tokio::test]
    async fn subscribed_receiver_is_woken_by_increment() {
        let counter = AwaitableCounter::default();
        let mut update_rx = counter.subscribe();

        counter.increment();

        let observed_within_deadline = timeout(Duration::from_secs(1), update_rx.changed())
            .await
            .expect("an increment must notify the subscriber within the deadline");

        assert!(observed_within_deadline.is_ok());
    }

    #[tokio::test]
    async fn wait_for_zero_returns_immediately_when_already_zero() {
        let counter = AwaitableCounter::default();

        counter.wait_for_zero().await;
    }

    #[tokio::test]
    async fn wait_for_zero_completes_after_decrement_to_zero() {
        let counter = Arc::new(AwaitableCounter::default());
        let guard = counter.increment_with_guard();

        let waiter_counter = counter.clone();
        let mut waiter =
            tokio_test::task::spawn(async move { waiter_counter.wait_for_zero().await });

        assert!(
            waiter.poll().is_pending(),
            "wait_for_zero must pend while the count is above zero"
        );

        drop(guard);

        assert!(
            waiter.is_woken(),
            "the decrement to zero must wake the waiter"
        );

        waiter.await;
    }

    #[tokio::test]
    async fn wait_for_zero_stays_pending_until_last_decrement() {
        let counter = Arc::new(AwaitableCounter::default());
        let first_guard = counter.increment_with_guard();
        let second_guard = counter.increment_with_guard();

        let waiter_counter = counter.clone();
        let mut waiter =
            tokio_test::task::spawn(async move { waiter_counter.wait_for_zero().await });

        assert!(waiter.poll().is_pending());

        drop(first_guard);

        assert!(waiter.is_woken());
        assert!(
            waiter.poll().is_pending(),
            "wait_for_zero must stay pending while one guard is still held"
        );

        drop(second_guard);

        assert!(waiter.is_woken());

        waiter.await;
    }
}
