use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use paddler::balancer::agent_controller_pool::AgentControllerPool;
use paddler::produces_snapshot::ProducesSnapshot;
use paddler::service::Service;
use paddler_types::agent_controller_snapshot::AgentControllerSnapshot;
use tokio::sync::broadcast;
use tokio::sync::mpsc;

pub struct AgentMonitorService {
    pub agent_controller_pool: Arc<AgentControllerPool>,
    pub agent_snapshots_tx: mpsc::UnboundedSender<Vec<AgentControllerSnapshot>>,
}

fn collect_agent_snapshots(pool: &AgentControllerPool) -> Result<Vec<AgentControllerSnapshot>> {
    let pool_snapshot = pool.make_snapshot()?;
    let mut agents = pool_snapshot.agents;

    agents.sort_by(|left, right| {
        let left_name = left.name.as_deref().unwrap_or(&left.id);
        let right_name = right.name.as_deref().unwrap_or(&right.id);

        left_name.cmp(right_name)
    });

    Ok(agents)
}

#[async_trait]
impl Service for AgentMonitorService {
    fn name(&self) -> &'static str {
        "agent_monitor"
    }

    async fn run(&mut self, mut shutdown_rx: broadcast::Receiver<()>) -> Result<()> {
        loop {
            let snapshots = collect_agent_snapshots(&self.agent_controller_pool)?;

            if let Err(send_error) = self.agent_snapshots_tx.send(snapshots) {
                log::warn!("Agent snapshots receiver dropped: {send_error}");

                break;
            }

            tokio::select! {
                _ = self.agent_controller_pool.update_notifier.notified() => {}
                _ = shutdown_rx.recv() => {
                    break;
                }
            }
        }

        Ok(())
    }
}
