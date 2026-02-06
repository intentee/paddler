use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use dashmap::DashMap;
use tokio::sync::Notify;

use super::agent_controller::AgentController;
use super::agent_controller_pool_total_slots::AgentControllerPoolTotalSlots;
use crate::agent_desired_state::AgentDesiredState;
use crate::balancer::agent_controller_pool_snapshot::AgentControllerPoolSnapshot;
use crate::balancer::agent_controller_snapshot::AgentControllerSnapshot;
use crate::produces_snapshot::ProducesSnapshot;
use crate::sets_desired_state::SetsDesiredState;

pub struct AgentControllerPool {
    pub agents: DashMap<String, Arc<AgentController>>,
    pub update_notifier: Arc<Notify>,
}

impl AgentControllerPool {
    pub fn take_least_busy_agent_controller(&self) -> Option<Arc<AgentController>> {
        let agent_controller: Option<Arc<AgentController>> = self
            .agents
            .iter()
            .map(|entry| entry.value().clone())
            .filter(|agent| agent.slots_processing.get() < agent.slots_total.get())
            .min_by_key(|agent| agent.slots_processing.get());

        if let Some(agent_controller) = agent_controller {
            agent_controller.slots_processing.increment();
            self.update_notifier.notify_waiters();

            return Some(agent_controller);
        }

        None
    }

    pub fn get_agent_controller(&self, agent_id: &str) -> Option<Arc<AgentController>> {
        self.agents.get(agent_id).map(|entry| entry.value().clone())
    }

    pub fn register_agent_controller(
        &self,
        agent_id: String,
        agent: Arc<AgentController>,
    ) -> Result<()> {
        if self.agents.insert(agent_id, agent).is_none() {
            self.update_notifier.notify_waiters();

            Ok(())
        } else {
            Err(anyhow::anyhow!("AgentController already registered"))
        }
    }

    pub fn remove_agent_controller(&self, agent_id: &str) -> Result<bool> {
        if self.agents.remove(agent_id).is_some() {
            self.update_notifier.notify_waiters();

            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn total_slots(&self) -> AgentControllerPoolTotalSlots {
        let mut slots_processing = 0;
        let mut slots_total = 0;

        for entry in self.agents.iter() {
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
        AgentControllerPool {
            agents: DashMap::new(),
            update_notifier: Arc::new(Notify::new()),
        }
    }
}

impl ProducesSnapshot for AgentControllerPool {
    type Snapshot = AgentControllerPoolSnapshot;

    fn make_snapshot(&self) -> Result<Self::Snapshot> {
        let mut agents: Vec<AgentControllerSnapshot> = Vec::with_capacity(self.agents.len());

        for entry in self.agents.iter() {
            let agent_controller = entry.value();

            agents.push(agent_controller.make_snapshot()?);
        }

        Ok(AgentControllerPoolSnapshot { agents })
    }
}

#[async_trait]
impl SetsDesiredState for AgentControllerPool {
    async fn set_desired_state(&self, desired_state: AgentDesiredState) -> Result<()> {
        for agent in self.agents.iter() {
            let agent_controller = agent.value();

            agent_controller
                .set_desired_state(desired_state.clone())
                .await?;
        }

        Ok(())
    }
}
