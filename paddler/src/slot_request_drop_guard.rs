use std::sync::Arc;

use crate::dispenses_slots::DispensesSlots;
use crate::slot_aggregated_status::SlotAggregatedStatus;

pub struct SlotRequestDropGuard {
    slot_aggregated_status: Arc<SlotAggregatedStatus>,
}

impl SlotRequestDropGuard {
    #[must_use]
    pub fn new(slot_aggregated_status: Arc<SlotAggregatedStatus>) -> Self {
        slot_aggregated_status.take_slot();

        Self {
            slot_aggregated_status,
        }
    }
}

impl Drop for SlotRequestDropGuard {
    fn drop(&mut self) {
        self.slot_aggregated_status.release_slot();
    }
}
