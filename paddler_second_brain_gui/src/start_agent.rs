use paddler_types::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

use crate::start_agent_services::start_agent_services;

pub async fn start_agent(
    agent_name: Option<String>,
    management_address: String,
    slots: i32,
    agent_status_tx: mpsc::UnboundedSender<SlotAggregatedStatusSnapshot>,
    shutdown_rx: oneshot::Receiver<()>,
) -> anyhow::Result<()> {
    let (result_tx, result_rx) = oneshot::channel();

    std::thread::spawn(move || {
        let system = actix_web::rt::System::new();
        let result = system.block_on(start_agent_services(
            agent_name,
            management_address,
            slots,
            agent_status_tx,
            shutdown_rx,
        ));
        if let Err(unsent_result) = result_tx.send(result) {
            log::error!("Failed to send agent result: {unsent_result:?}");
        }
    });

    result_rx
        .await
        .map_err(|error| anyhow::anyhow!("Agent thread terminated: {error}"))?
}
