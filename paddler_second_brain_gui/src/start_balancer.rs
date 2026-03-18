use std::net::IpAddr;

use paddler_types::balancer_desired_state::BalancerDesiredState;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

use crate::start_balancer_services::start_balancer_services;

pub async fn start_balancer(
    bind_ip: IpAddr,
    management_port: u16,
    initial_desired_state: BalancerDesiredState,
    agent_count_tx: mpsc::UnboundedSender<usize>,
    shutdown_rx: oneshot::Receiver<()>,
) -> anyhow::Result<()> {
    let (result_tx, result_rx) = oneshot::channel();

    std::thread::spawn(move || {
        let system = actix_web::rt::System::new();
        let result = system.block_on(start_balancer_services(
            bind_ip,
            management_port,
            initial_desired_state,
            agent_count_tx,
            shutdown_rx,
        ));
        if let Err(unsent_result) = result_tx.send(result) {
            log::error!("Failed to send balancer result: {unsent_result:?}");
        }
    });

    result_rx
        .await
        .map_err(|error| anyhow::anyhow!("Balancer thread terminated: {error}"))?
}
