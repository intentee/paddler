use std::sync::Arc;

use crate::slot_aggregated_status::SlotAggregatedStatus;

pub struct SlotAggregatedStatusManager {
    pub slot_aggregated_status: Arc<SlotAggregatedStatus>,
}

impl SlotAggregatedStatusManager {
    #[must_use]
    pub fn new(desired_slots_total: i32) -> Self {
        Self {
            slot_aggregated_status: Arc::new(SlotAggregatedStatus::new(desired_slots_total)),
        }
    }

    pub fn reset(&self) {
        self.slot_aggregated_status.reset();
    }
}
