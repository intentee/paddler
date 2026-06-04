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
    use std::time::Duration;

    use tokio_util::sync::CancellationToken;

    use crate::drain_in_flight_requests::drain_in_flight_requests;
    use crate::slot_aggregated_status_manager::SlotAggregatedStatusManager;
    use crate::slot_guard::SlotGuard;

    #[tokio::test]
    async fn increments_slot_on_construct_and_releases_on_drop() {
        let slot_aggregated_status_manager = Arc::new(SlotAggregatedStatusManager::new(4));

        assert_eq!(
            slot_aggregated_status_manager
                .slot_aggregated_status
                .slots_processing_count(),
            0
        );

        {
            let _guard = SlotGuard::new(
                slot_aggregated_status_manager
                    .slot_aggregated_status
                    .clone(),
            );

            assert_eq!(
                slot_aggregated_status_manager
                    .slot_aggregated_status
                    .slots_processing_count(),
                1
            );
        }

        assert_eq!(
            slot_aggregated_status_manager
                .slot_aggregated_status
                .slots_processing_count(),
            0
        );
    }

    #[tokio::test]
    async fn drain_in_flight_requests_blocks_until_guard_dropped() {
        let slot_aggregated_status_manager = Arc::new(SlotAggregatedStatusManager::new(4));
        let shutdown = CancellationToken::new();

        let guard = SlotGuard::new(
            slot_aggregated_status_manager
                .slot_aggregated_status
                .clone(),
        );

        let manager_for_drain = slot_aggregated_status_manager.clone();
        let shutdown_for_drain = shutdown.clone();
        let mut drain_task = tokio::spawn(async move {
            drain_in_flight_requests(&manager_for_drain, &shutdown_for_drain).await
        });

        let blocking_window = Duration::from_millis(50);
        let timeout_result = tokio::time::timeout(blocking_window, &mut drain_task).await;
        assert!(
            timeout_result.is_err(),
            "drain_in_flight_requests returned while a SlotGuard was still held"
        );

        drop(guard);

        let unblock_window = Duration::from_millis(500);
        tokio::time::timeout(unblock_window, drain_task)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
    }
}
