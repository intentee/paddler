use std::sync::Arc;

use crate::dispenses_slots::DispensesSlots as _;
use crate::slot_aggregated_status::SlotAggregatedStatus;

pub struct SlotGuard {
    slot_aggregated_status: Arc<SlotAggregatedStatus>,
}

impl SlotGuard {
    #[must_use]
    pub fn new(slot_aggregated_status: Arc<SlotAggregatedStatus>) -> Self {
        slot_aggregated_status.take_slot();

        Self {
            slot_aggregated_status,
        }
    }
}

impl Drop for SlotGuard {
    fn drop(&mut self) {
        self.slot_aggregated_status.release_slot();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::slot_aggregated_status::SlotAggregatedStatus;
    use crate::slot_guard::SlotGuard;

    #[tokio::test]
    async fn increments_slot_on_construct_and_releases_on_drop() {
        let slot_aggregated_status = Arc::new(SlotAggregatedStatus::new(4));

        assert_eq!(slot_aggregated_status.slots_processing_count(), 0);

        {
            let _guard = SlotGuard::new(slot_aggregated_status.clone());

            assert_eq!(slot_aggregated_status.slots_processing_count(), 1);
        }

        assert_eq!(slot_aggregated_status.slots_processing_count(), 0);
    }
}
