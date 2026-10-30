use std::num::NonZeroU16;
use std::sync::Arc;

use crate::dispenses_slots::DispensesSlots as _;
use crate::slot_aggregated_status::SlotAggregatedStatus;

pub struct SlotGuard {
    slot_aggregated_status: Arc<SlotAggregatedStatus>,
    slots: NonZeroU16,
}

impl SlotGuard {
    #[must_use]
    pub fn new(slot_aggregated_status: Arc<SlotAggregatedStatus>, slots: NonZeroU16) -> Self {
        slot_aggregated_status.take_slots(slots);

        Self {
            slot_aggregated_status,
            slots,
        }
    }

    #[must_use]
    pub const fn slots(&self) -> NonZeroU16 {
        self.slots
    }
}

impl Drop for SlotGuard {
    fn drop(&mut self) {
        self.slot_aggregated_status.release_slots(self.slots);
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU16;
    use std::sync::Arc;

    use crate::slot_aggregated_status::SlotAggregatedStatus;
    use crate::slot_guard::SlotGuard;

    #[tokio::test]
    async fn holds_its_reserved_slots_until_dropped() {
        let slot_aggregated_status = Arc::new(SlotAggregatedStatus::new(4));

        {
            let guard = SlotGuard::new(slot_aggregated_status.clone(), NonZeroU16::new(3).unwrap());

            assert_eq!(slot_aggregated_status.slots_processing_count(), 3);
            assert_eq!(guard.slots(), NonZeroU16::new(3).unwrap());
        }

        assert_eq!(slot_aggregated_status.slots_processing_count(), 0);
    }
}
