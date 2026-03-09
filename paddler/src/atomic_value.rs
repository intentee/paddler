use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicI32;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

pub struct AtomicValue<TAtomic> {
    value: TAtomic,
}

impl AtomicValue<AtomicBool> {
    pub fn new(initial: bool) -> Self {
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
        if self.get() != value {
            self.set(value);

            true
        } else {
            false
        }
    }
}

impl AtomicValue<AtomicI32> {
    pub fn new(initial: i32) -> Self {
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
        if self.get() != value {
            self.set(value);

            true
        } else {
            false
        }
    }
}

impl AtomicValue<AtomicUsize> {
    pub fn new(initial: usize) -> Self {
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
        if self.get() != value {
            self.set(value);

            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod atomic_bool_tests {
        use super::*;

        #[test]
        fn set_check_returns_true_on_change() {
            let atomic = AtomicValue::<AtomicBool>::new(false);

            assert!(atomic.set_check(true));
            assert!(atomic.get());
        }

        #[test]
        fn set_check_returns_false_on_same_value() {
            let atomic = AtomicValue::<AtomicBool>::new(true);

            assert!(!atomic.set_check(true));
        }
    }

    mod atomic_i32_tests {
        use super::*;

        #[test]
        fn set_check_returns_true_on_change() {
            let atomic = AtomicValue::<AtomicI32>::new(0);

            assert!(atomic.set_check(5));
            assert_eq!(atomic.get(), 5);
        }

        #[test]
        fn set_check_returns_false_on_same_value() {
            let atomic = AtomicValue::<AtomicI32>::new(5);

            assert!(!atomic.set_check(5));
        }

        #[test]
        fn compare_and_swap_succeeds_when_current_matches() {
            let atomic = AtomicValue::<AtomicI32>::new(10);

            assert!(atomic.compare_and_swap(10, 20));
            assert_eq!(atomic.get(), 20);
        }

        #[test]
        fn compare_and_swap_fails_when_current_does_not_match() {
            let atomic = AtomicValue::<AtomicI32>::new(10);

            assert!(!atomic.compare_and_swap(5, 20));
            assert_eq!(atomic.get(), 10);
        }

        #[test]
        fn increment_adds_one() {
            let atomic = AtomicValue::<AtomicI32>::new(3);

            atomic.increment();

            assert_eq!(atomic.get(), 4);
        }

        #[test]
        fn decrement_subtracts_one() {
            let atomic = AtomicValue::<AtomicI32>::new(3);

            atomic.decrement();

            assert_eq!(atomic.get(), 2);
        }

        #[test]
        fn reset_sets_to_zero() {
            let atomic = AtomicValue::<AtomicI32>::new(42);

            atomic.reset();

            assert_eq!(atomic.get(), 0);
        }
    }

    mod atomic_usize_tests {
        use super::*;

        #[test]
        fn set_check_returns_true_on_change() {
            let atomic = AtomicValue::<AtomicUsize>::new(0);

            assert!(atomic.set_check(5));
            assert_eq!(atomic.get(), 5);
        }

        #[test]
        fn set_check_returns_false_on_same_value() {
            let atomic = AtomicValue::<AtomicUsize>::new(5);

            assert!(!atomic.set_check(5));
        }

        #[test]
        fn increment_by_adds_given_amount() {
            let atomic = AtomicValue::<AtomicUsize>::new(10);

            atomic.increment_by(7);

            assert_eq!(atomic.get(), 17);
        }
    }
}
