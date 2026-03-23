use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use paddler::produces_snapshot::ProducesSnapshot;
use paddler::service::Service;
use paddler::slot_aggregated_status::SlotAggregatedStatus;
use paddler_types::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;
use tokio::sync::broadcast;
use tokio::sync::mpsc;

pub struct AgentStatusMonitorService {
    pub agent_status_tx: mpsc::UnboundedSender<SlotAggregatedStatusSnapshot>,
    pub slot_aggregated_status: Arc<SlotAggregatedStatus>,
}

#[async_trait]
impl Service for AgentStatusMonitorService {
    fn name(&self) -> &'static str {
        "agent_status_monitor"
    }

    async fn run(&mut self, mut shutdown_rx: broadcast::Receiver<()>) -> Result<()> {
        let mut previous_version: Option<i32> = None;

        loop {
            let snapshot = self.slot_aggregated_status.make_snapshot()?;

            let has_changed = previous_version
                .map(|previous| previous != snapshot.version)
                .unwrap_or(true);

            if has_changed {
                previous_version = Some(snapshot.version);

                if let Err(send_error) = self.agent_status_tx.send(snapshot) {
                    log::warn!("Agent status receiver dropped: {send_error}");

                    break;
                }
            }

            tokio::select! {
                _ = self.slot_aggregated_status.update_notifier.notified() => {}
                _ = shutdown_rx.recv() => {
                    break;
                }
            }
        }

        Ok(())
    }
}
