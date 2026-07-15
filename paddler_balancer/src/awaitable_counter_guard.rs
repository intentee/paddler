use std::sync::Arc;

use crate::awaitable_counter::AwaitableCounter;

pub struct AwaitableCounterGuard {
    counter: Arc<AwaitableCounter>,
}

impl AwaitableCounterGuard {
    pub const fn new(counter: Arc<AwaitableCounter>) -> Self {
        Self { counter }
    }
}

impl Drop for AwaitableCounterGuard {
    fn drop(&mut self) {
        self.counter.decrement();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drop_decrements_counter() {
        let counter = Arc::new(AwaitableCounter::default());

        counter.increment();
        counter.increment();

        let guard = AwaitableCounterGuard::new(counter.clone());

        assert_eq!(counter.get(), 2);

        drop(guard);

        assert_eq!(counter.get(), 1);
    }
}
