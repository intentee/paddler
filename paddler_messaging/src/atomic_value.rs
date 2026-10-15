use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

pub struct AtomicValue<TAtomic> {
    value: TAtomic,
}

impl AtomicValue<AtomicBool> {
    #[must_use]
    pub const fn new(initial: bool) -> Self {
        Self {
            value: AtomicBool::new(initial),
        }
    }

    pub fn get(&self) -> bool {
        self.value.load(Ordering::SeqCst)
    }

    pub fn set(&self, value: bool) {
        self.value.store(value, Ordering::SeqCst);
    }
}

impl AtomicValue<AtomicU64> {
    #[must_use]
    pub const fn new(initial: u64) -> Self {
        Self {
            value: AtomicU64::new(initial),
        }
    }

    pub fn compare_and_swap(&self, current: u64, new: u64) -> bool {
        self.value
            .compare_exchange(current, new, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }

    pub fn decrement(&self) {
        self.value.fetch_sub(1, Ordering::SeqCst);
    }

    pub fn get(&self) -> u64 {
        self.value.load(Ordering::SeqCst)
    }

    pub fn increment(&self) {
        self.value.fetch_add(1, Ordering::SeqCst);
    }

    pub fn increment_below(&self, limit: u64) -> bool {
        self.value
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
                (current < limit).then_some(current + 1)
            })
            .is_ok()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicU64;

    use super::AtomicValue;

    #[test]
    fn compare_and_swap_replaces_only_the_expected_value() {
        let value = AtomicValue::<AtomicU64>::new(1);

        assert!(value.compare_and_swap(1, 5));
        assert!(!value.compare_and_swap(1, 9));
        assert_eq!(value.get(), 5);
    }
}
