use std::net::SocketAddr;

use paddler_types::agent_controller_snapshot::AgentControllerSnapshot;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

use crate::start_balancer_services::start_balancer_services;

pub async fn start_balancer(
    management_addr: SocketAddr,
    inference_addr: SocketAddr,
    initial_desired_state: BalancerDesiredState,
    agent_snapshots_tx: mpsc::UnboundedSender<Vec<AgentControllerSnapshot>>,
    shutdown_rx: oneshot::Receiver<()>,
) -> anyhow::Result<()> {
    tokio::task::spawn_blocking(move || {
        let system = actix_web::rt::System::new();

        system.block_on(start_balancer_services(
            management_addr,
            inference_addr,
            initial_desired_state,
            agent_snapshots_tx,
            shutdown_rx,
        ))
    })
    .await
    .map_err(|error| anyhow::anyhow!("Balancer task panicked: {error}"))?
}
