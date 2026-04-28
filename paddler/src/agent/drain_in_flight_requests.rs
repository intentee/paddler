use std::sync::Arc;

use anyhow::Context as _;
use anyhow::Result;
use log::info;
use tokio_util::sync::CancellationToken;

use crate::slot_aggregated_status_manager::SlotAggregatedStatusManager;
use crate::subscribes_to_updates::SubscribesToUpdates as _;

pub async fn drain_in_flight_requests(
    slot_aggregated_status_manager: &Arc<SlotAggregatedStatusManager>,
    shutdown: &CancellationToken,
) -> Result<()> {
    let mut update_rx = slot_aggregated_status_manager
        .slot_aggregated_status
        .subscribe_to_updates();

    while slot_aggregated_status_manager
        .slot_aggregated_status
        .slots_processing_count()
        > 0
    {
        tokio::select! {
            () = shutdown.cancelled() => {
                info!("Shutdown during drain, proceeding immediately");

                return Ok(());
            }
            changed = update_rx.changed() => {
                changed.context("update channel closed while draining in-flight requests")?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use tokio_util::sync::CancellationToken;

    use crate::dispenses_slots::DispensesSlots;
    use crate::slot_aggregated_status_manager::SlotAggregatedStatusManager;

    use super::drain_in_flight_requests;

    fn create_status_manager(desired_slots: i32) -> Arc<SlotAggregatedStatusManager> {
        Arc::new(SlotAggregatedStatusManager::new(desired_slots))
    }

    #[tokio::test]
    async fn returns_immediately_when_no_slots_processing() -> anyhow::Result<()> {
        let slot_aggregated_status_manager = create_status_manager(4);
        let shutdown = CancellationToken::new();

        drain_in_flight_requests(&slot_aggregated_status_manager, &shutdown).await?;

        Ok(())
    }

    #[tokio::test]
    async fn waits_for_processing_slots_to_reach_zero() -> anyhow::Result<()> {
        let slot_aggregated_status_manager = create_status_manager(4);
        let shutdown = CancellationToken::new();

        slot_aggregated_status_manager
            .slot_aggregated_status
            .take_slot();

        let status = slot_aggregated_status_manager
            .slot_aggregated_status
            .clone();
        let release_handle = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            status.release_slot();
        });

        drain_in_flight_requests(&slot_aggregated_status_manager, &shutdown).await?;

        assert_eq!(
            slot_aggregated_status_manager
                .slot_aggregated_status
                .slots_processing_count(),
            0
        );

        release_handle.await?;

        Ok(())
    }

    #[tokio::test]
    async fn aborts_on_shutdown_signal() -> anyhow::Result<()> {
        let slot_aggregated_status_manager = create_status_manager(4);
        let shutdown = CancellationToken::new();

        slot_aggregated_status_manager
            .slot_aggregated_status
            .take_slot();

        let shutdown_trigger = shutdown.clone();
        let shutdown_handle = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            shutdown_trigger.cancel();
        });

        drain_in_flight_requests(&slot_aggregated_status_manager, &shutdown).await?;

        assert_eq!(
            slot_aggregated_status_manager
                .slot_aggregated_status
                .slots_processing_count(),
            1,
        );

        shutdown_handle.await?;

        Ok(())
    }
}
