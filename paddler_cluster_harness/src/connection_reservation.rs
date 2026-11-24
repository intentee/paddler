use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

pub struct ConnectionReservation {
    pub held_connections: Arc<AtomicUsize>,
}

impl Drop for ConnectionReservation {
    fn drop(&mut self) {
        self.held_connections.fetch_sub(1, Ordering::AcqRel);
    }
}
