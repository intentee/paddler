use std::sync::Arc;

use crate::slot_aggregated_status::SlotAggregatedStatus;
use crate::slot_status::SlotStatus;

pub struct SlotAggregatedStatusManager {
    pub slot_aggregated_status: Arc<SlotAggregatedStatus>,
}

impl SlotAggregatedStatusManager {
    pub fn new(desired_slots_total: i32) -> Self {
        SlotAggregatedStatusManager {
            slot_aggregated_status: Arc::new(SlotAggregatedStatus::new(desired_slots_total)),
        }
    }

    pub fn bind_slot_status(&self) -> Arc<SlotStatus> {
        Arc::new(SlotStatus::new(self.slot_aggregated_status.clone()))
    }

    pub fn reset(&self) {
        self.slot_aggregated_status.reset();
    }
}
