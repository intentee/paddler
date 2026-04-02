use std::sync::Arc;

use log::info;
use tokio::sync::broadcast;

use crate::slot_aggregated_status_manager::SlotAggregatedStatusManager;

pub async fn drain_in_flight_requests(
    slot_aggregated_status_manager: &Arc<SlotAggregatedStatusManager>,
    shutdown: &mut broadcast::Receiver<()>,
) {
    loop {
        let notified = slot_aggregated_status_manager
            .slot_aggregated_status
            .update_notifier
            .notified();
        tokio::pin!(notified);

        if slot_aggregated_status_manager
            .slot_aggregated_status
            .slots_processing_count()
            == 0
        {
            break;
        }

        tokio::select! {
            _ = shutdown.recv() => {
                info!("Shutdown during drain, proceeding immediately");

                break;
            }
            () = &mut notified => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use tokio::sync::broadcast;

    use crate::dispenses_slots::DispensesSlots;
    use crate::slot_aggregated_status_manager::SlotAggregatedStatusManager;

    use super::drain_in_flight_requests;

    fn create_status_manager(desired_slots: i32) -> Arc<SlotAggregatedStatusManager> {
        Arc::new(SlotAggregatedStatusManager::new(desired_slots))
    }

    #[tokio::test]
    async fn returns_immediately_when_no_slots_processing() {
        let slot_aggregated_status_manager = create_status_manager(4);
        let (shutdown_tx, mut shutdown_rx) = broadcast::channel::<()>(1);

        drain_in_flight_requests(&slot_aggregated_status_manager, &mut shutdown_rx).await;

        drop(shutdown_tx);
    }

    #[tokio::test]
    async fn waits_for_processing_slots_to_reach_zero() -> Result<(), tokio::task::JoinError> {
        let slot_aggregated_status_manager = create_status_manager(4);
        let (shutdown_tx, mut shutdown_rx) = broadcast::channel::<()>(1);

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

        drain_in_flight_requests(&slot_aggregated_status_manager, &mut shutdown_rx).await;

        assert_eq!(
            slot_aggregated_status_manager
                .slot_aggregated_status
                .slots_processing_count(),
            0
        );

        release_handle.await?;
        drop(shutdown_tx);

        Ok(())
    }

    #[tokio::test]
    async fn aborts_on_shutdown_signal() -> Result<(), tokio::task::JoinError> {
        let slot_aggregated_status_manager = create_status_manager(4);
        let (shutdown_tx, mut shutdown_rx) = broadcast::channel::<()>(1);

        slot_aggregated_status_manager
            .slot_aggregated_status
            .take_slot();

        let shutdown_handle = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            let _ = shutdown_tx.send(());
        });

        drain_in_flight_requests(&slot_aggregated_status_manager, &mut shutdown_rx).await;

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
