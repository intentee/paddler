use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use paddler::balancer::agent_controller_pool::AgentControllerPool;
use paddler::service::Service;
use tokio::sync::broadcast;
use tokio::sync::mpsc;

pub struct AgentMonitorService {
    pub agent_controller_pool: Arc<AgentControllerPool>,
    pub agent_count_tx: mpsc::UnboundedSender<usize>,
}

#[async_trait]
impl Service for AgentMonitorService {
    fn name(&self) -> &'static str {
        "agent_monitor"
    }

    async fn run(&mut self, mut shutdown_rx: broadcast::Receiver<()>) -> Result<()> {
        let mut previous_count: Option<usize> = None;

        loop {
            let count = self.agent_controller_pool.agents.len();

            let has_changed = previous_count
                .map(|previous| previous != count)
                .unwrap_or(true);

            if has_changed {
                if let Err(send_error) = self.agent_count_tx.send(count) {
                    log::warn!("Agent count receiver dropped: {send_error}");

                    break;
                }

                previous_count = Some(count);
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
