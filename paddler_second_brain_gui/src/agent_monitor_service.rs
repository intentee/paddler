use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use paddler::balancer::agent_controller_pool::AgentControllerPool;
use paddler::service::Service;
use tokio::sync::broadcast;
use tokio::sync::mpsc;

pub struct AgentMonitorService {
    pub agent_controller_pool: Arc<AgentControllerPool>,
    pub agent_names_tx: mpsc::UnboundedSender<Vec<String>>,
}

fn collect_agent_names(pool: &AgentControllerPool) -> Vec<String> {
    let mut names: Vec<String> = pool
        .agents
        .iter()
        .map(|entry| {
            entry
                .value()
                .name
                .clone()
                .unwrap_or_else(|| entry.key().clone())
        })
        .collect();

    names.sort();

    names
}

#[async_trait]
impl Service for AgentMonitorService {
    fn name(&self) -> &'static str {
        "agent_monitor"
    }

    async fn run(&mut self, mut shutdown_rx: broadcast::Receiver<()>) -> Result<()> {
        loop {
            let names = collect_agent_names(&self.agent_controller_pool);

            if let Err(send_error) = self.agent_names_tx.send(names) {
                log::warn!("Agent names receiver dropped: {send_error}");

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
