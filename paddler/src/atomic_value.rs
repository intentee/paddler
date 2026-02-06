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
