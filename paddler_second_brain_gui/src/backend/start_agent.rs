use paddler_types::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

use super::start_agent_services::start_agent_services;

pub async fn start_agent(
    agent_name: Option<String>,
    management_address: String,
    slots: i32,
    agent_status_tx: mpsc::UnboundedSender<SlotAggregatedStatusSnapshot>,
    shutdown_rx: oneshot::Receiver<()>,
) -> anyhow::Result<()> {
    tokio::task::spawn_blocking(move || {
        let system = actix_web::rt::System::new();

        system.block_on(start_agent_services(
            agent_name,
            management_address,
            slots,
            agent_status_tx,
            shutdown_rx,
        ))
    })
    .await
    .map_err(|error| anyhow::anyhow!("Agent task panicked: {error}"))?
}
