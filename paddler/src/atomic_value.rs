use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicI32;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::AtomicUsize;
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

    pub fn set_check(&self, value: bool) -> bool {
        if self.get() == value {
            false
        } else {
            self.set(value);

            true
        }
    }
}

impl AtomicValue<AtomicI32> {
    #[must_use]
    pub const fn new(initial: i32) -> Self {
        Self {
            value: AtomicI32::new(initial),
        }
    }

    pub fn compare_and_swap(&self, current: i32, new: i32) -> bool {
        self.value
            .compare_exchange(current, new, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }

    pub fn decrement(&self) {
        self.value.fetch_sub(1, Ordering::SeqCst);
    }

    pub fn get(&self) -> i32 {
        self.value.load(Ordering::SeqCst)
    }

    pub fn increment(&self) {
        self.value.fetch_add(1, Ordering::SeqCst);
    }

    pub fn reset(&self) {
        self.value.store(0, Ordering::SeqCst);
    }

    pub fn set(&self, value: i32) {
        self.value.store(value, Ordering::SeqCst);
    }

    pub fn set_check(&self, value: i32) -> bool {
        if self.get() == value {
            false
        } else {
            self.set(value);

            true
        }
    }
}

impl AtomicValue<AtomicU64> {
    #[must_use]
    pub const fn new(initial: u64) -> Self {
        Self {
            value: AtomicU64::new(initial),
        }
    }

    pub fn get(&self) -> u64 {
        self.value.load(Ordering::SeqCst)
    }

    pub fn increment_by(&self, increment: u64) {
        self.value.fetch_add(increment, Ordering::SeqCst);
    }

    pub fn set(&self, value: u64) {
        self.value.store(value, Ordering::SeqCst);
    }

    pub fn set_check(&self, value: u64) -> bool {
        if self.get() == value {
            false
        } else {
            self.set(value);

            true
        }
    }
}

impl AtomicValue<AtomicUsize> {
    #[must_use]
    pub const fn new(initial: usize) -> Self {
        Self {
            value: AtomicUsize::new(initial),
        }
    }

    pub fn get(&self) -> usize {
        self.value.load(Ordering::SeqCst)
    }

    pub fn increment_by(&self, increment: usize) {
        self.value.fetch_add(increment, Ordering::SeqCst);
    }

    pub fn set(&self, value: usize) {
        self.value.store(value, Ordering::SeqCst);
    }

    pub fn set_check(&self, value: usize) -> bool {
        if self.get() == value {
            false
        } else {
            self.set(value);

            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bool_set_check_reports_and_applies_changes() {
        let value = AtomicValue::<AtomicBool>::new(false);

        assert!(!value.get());
        assert!(value.set_check(true));
        assert!(value.get());
        assert!(!value.set_check(true));

        value.set(false);

        assert!(!value.get());
    }

    #[test]
    fn i32_arithmetic_and_compare_and_swap() {
        let value = AtomicValue::<AtomicI32>::new(0);

        value.increment();
        value.increment();
        value.decrement();

        assert_eq!(value.get(), 1);
        assert!(value.compare_and_swap(1, 5));
        assert_eq!(value.get(), 5);
        assert!(!value.compare_and_swap(1, 9));
        assert_eq!(value.get(), 5);

        value.set(7);

        assert!(value.set_check(8));
        assert!(!value.set_check(8));

        value.reset();

        assert_eq!(value.get(), 0);
    }

    #[test]
    fn u64_increment_by_and_set_check() {
        let value = AtomicValue::<AtomicU64>::new(0);

        value.increment_by(10);

        assert_eq!(value.get(), 10);
        assert!(value.set_check(20));
        assert!(!value.set_check(20));

        value.set(0);

        assert_eq!(value.get(), 0);
    }

    #[test]
    fn usize_increment_by_and_set_check() {
        let value = AtomicValue::<AtomicUsize>::new(0);

        value.increment_by(3);

        assert_eq!(value.get(), 3);
        assert!(value.set_check(4));
        assert!(!value.set_check(4));

        value.set(0);

        assert_eq!(value.get(), 0);
    }
}
