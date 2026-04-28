use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use dashmap::DashMap;
use paddler_types::agent_controller_pool_snapshot::AgentControllerPoolSnapshot;
use paddler_types::agent_controller_snapshot::AgentControllerSnapshot;
use tokio::sync::watch;

use super::agent_controller::AgentController;
use super::agent_controller_pool_total_slots::AgentControllerPoolTotalSlots;
use crate::agent_desired_state::AgentDesiredState;
use crate::produces_snapshot::ProducesSnapshot;
use crate::sets_desired_state::SetsDesiredState;
use crate::subscribes_to_updates::SubscribesToUpdates;

pub struct AgentControllerPool {
    pub agents: DashMap<String, Arc<AgentController>>,
    update_tx: watch::Sender<()>,
}

impl AgentControllerPool {
    #[must_use]
    pub fn take_least_busy_agent_controller(&self) -> Option<Arc<AgentController>> {
        let agent_controller: Option<Arc<AgentController>> = self
            .agents
            .iter()
            .map(|entry| entry.value().clone())
            .filter(|agent| agent.slots_processing.get() < agent.slots_total.get())
            .min_by_key(|agent| agent.slots_processing.get());

        if let Some(agent_controller) = agent_controller {
            agent_controller.slots_processing.increment();
            self.update_tx.send_replace(());

            return Some(agent_controller);
        }

        None
    }

    #[must_use]
    pub fn get_agent_controller(&self, agent_id: &str) -> Option<Arc<AgentController>> {
        self.agents.get(agent_id).map(|entry| entry.value().clone())
    }

    pub fn register_agent_controller(
        &self,
        agent_id: String,
        agent: Arc<AgentController>,
    ) -> Result<()> {
        if self.agents.insert(agent_id, agent).is_none() {
            self.update_tx.send_replace(());

            Ok(())
        } else {
            Err(anyhow::anyhow!("AgentController already registered"))
        }
    }

    pub fn remove_agent_controller(&self, agent_id: &str) -> Result<bool> {
        if self.agents.remove(agent_id).is_some() {
            self.update_tx.send_replace(());

            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn signal_update(&self) {
        self.update_tx.send_replace(());
    }

    #[must_use]
    pub fn total_slots(&self) -> AgentControllerPoolTotalSlots {
        let mut slots_processing = 0;
        let mut slots_total = 0;

        for entry in &self.agents {
            let agent = entry.value();

            slots_processing += agent.slots_processing.get();
            slots_total += agent.slots_total.get();
        }

        AgentControllerPoolTotalSlots {
            slots_processing,
            slots_total,
        }
    }
}

impl Default for AgentControllerPool {
    fn default() -> Self {
        let (update_tx, _initial_rx) = watch::channel(());

        Self {
            agents: DashMap::new(),
            update_tx,
        }
    }
}

impl SubscribesToUpdates for AgentControllerPool {
    fn subscribe_to_updates(&self) -> watch::Receiver<()> {
        self.update_tx.subscribe()
    }
}

impl ProducesSnapshot for AgentControllerPool {
    type Snapshot = AgentControllerPoolSnapshot;

    fn make_snapshot(&self) -> Result<Self::Snapshot> {
        let mut agents: Vec<AgentControllerSnapshot> = Vec::with_capacity(self.agents.len());

        for entry in &self.agents {
            let agent_controller = entry.value();

            agents.push(agent_controller.make_snapshot()?);
        }

        Ok(AgentControllerPoolSnapshot { agents })
    }
}

#[async_trait]
impl SetsDesiredState for AgentControllerPool {
    async fn set_desired_state(&self, desired_state: AgentDesiredState) -> Result<()> {
        for agent in &self.agents {
            let agent_controller = agent.value();

            agent_controller
                .set_desired_state(desired_state.clone())
                .await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::sync::watch;
    use tokio::time::timeout;

    #[tokio::test]
    async fn watch_receiver_observes_send_fired_before_changed_await() {
        let (update_tx, mut update_rx) = watch::channel(());

        update_tx.send_replace(());

        assert!(
            timeout(Duration::from_secs(1), update_rx.changed())
                .await
                .is_ok(),
            "watch::Receiver must observe a send fired before .changed() is awaited"
        );
    }
}
