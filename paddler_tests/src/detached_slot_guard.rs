use std::sync::Arc;

use paddler_agent::slot_aggregated_status::SlotAggregatedStatus;
use paddler_agent::slot_guard::SlotGuard;

#[must_use]
pub fn detached_slot_guard() -> SlotGuard {
    SlotGuard::new(Arc::new(SlotAggregatedStatus::new(1)))
}
