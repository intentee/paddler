use std::num::NonZeroUsize;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use tokio::sync::Mutex;

use crate::inference_socket::connection_slot::ConnectionSlot;

pub struct RoundRobinConnectionSlots {
    capacity: NonZeroUsize,
    first_connection_slot: Mutex<ConnectionSlot>,
    next_slot_index: AtomicUsize,
    other_connection_slots: Vec<Mutex<ConnectionSlot>>,
}

impl RoundRobinConnectionSlots {
    #[must_use]
    pub fn new(capacity: NonZeroUsize) -> Self {
        Self {
            capacity,
            first_connection_slot: Mutex::new(ConnectionSlot::Empty),
            next_slot_index: AtomicUsize::new(0),
            other_connection_slots: (1..capacity.get())
                .map(|_slot_index| Mutex::new(ConnectionSlot::Empty))
                .collect(),
        }
    }

    pub fn next_connection_slot(&self) -> &Mutex<ConnectionSlot> {
        let slot_position = self.next_slot_index.fetch_add(1, Ordering::Relaxed) % self.capacity;

        self.other_connection_slots.iter().take(slot_position).fold(
            &self.first_connection_slot,
            |_preceding_connection_slot, connection_slot| connection_slot,
        )
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;
    use std::ptr;

    use super::RoundRobinConnectionSlots;

    #[test]
    fn hands_out_every_slot_in_order_and_wraps_around_to_the_first() {
        let connection_slots = RoundRobinConnectionSlots::new(
            NonZeroUsize::new(2).expect("a capacity of two must be non-zero"),
        );
        let first_connection_slot = &connection_slots.first_connection_slot;
        let second_connection_slot = connection_slots
            .other_connection_slots
            .first()
            .expect("a capacity of two must hold a second slot");

        assert!(ptr::eq(
            connection_slots.next_connection_slot(),
            first_connection_slot
        ));
        assert!(ptr::eq(
            connection_slots.next_connection_slot(),
            second_connection_slot
        ));
        assert!(ptr::eq(
            connection_slots.next_connection_slot(),
            first_connection_slot
        ));
    }
}
