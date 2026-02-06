use std::sync::Arc;

use crate::dispenses_slots::DispensesSlots as _;
use crate::slot_status::SlotStatus;

pub struct SlotRequestDropGuard {
    slot_status: Arc<SlotStatus>,
}

impl SlotRequestDropGuard {
    pub fn new(slot_status: Arc<SlotStatus>) -> Self {
        Self { slot_status }
    }
}

impl Drop for SlotRequestDropGuard {
    fn drop(&mut self) {
        self.slot_status.release_slot();
    }
}
