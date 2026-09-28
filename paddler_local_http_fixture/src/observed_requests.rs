use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;

use tokio::sync::watch;

pub struct ObservedRequests {
    count: AtomicU32,
    last_range_header: watch::Sender<Option<Vec<u8>>>,
}

impl ObservedRequests {
    pub fn new() -> Self {
        Self {
            count: AtomicU32::new(0),
            last_range_header: watch::Sender::new(None),
        }
    }

    pub fn record(&self, range_header: Option<Vec<u8>>) {
        self.last_range_header.send_replace(range_header);
        self.count.fetch_add(1, Ordering::SeqCst);
    }

    pub fn count(&self) -> u32 {
        self.count.load(Ordering::SeqCst)
    }

    pub fn last_range_header(&self) -> Option<Vec<u8>> {
        self.last_range_header.borrow().clone()
    }
}
