use std::sync::atomic::AtomicU64;

use tokio::sync::watch;

use paddler_messaging::atomic_value::AtomicValue;

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
        self.count.subtract(1);
        self.update_tx.send_replace(());
    }

    pub fn get(&self) -> u64 {
        self.count.get()
    }

    pub fn try_increment(&self) -> bool {
        let incremented = self.count.increment_below(self.max_buffered_requests);

        if incremented {
            self.update_tx.send_replace(());
        }

        incremented
    }
}

#[cfg(test)]
mod tests {
    use tokio::sync::watch;

    use super::BufferedRequestCounter;

    #[test]
    fn starts_at_zero() {
        let (update_tx, _initial_rx) = watch::channel(());

        assert_eq!(BufferedRequestCounter::new(update_tx, 1).get(), 0);
    }
}
