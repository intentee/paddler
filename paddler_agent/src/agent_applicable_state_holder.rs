use anyhow::Result;
use parking_lot::RwLock;
use tokio::sync::watch;

use crate::agent_applicable_state::AgentApplicableState;

pub struct AgentApplicableStateHolder {
    agent_applicable_state: RwLock<Option<AgentApplicableState>>,
    change_notifier: watch::Sender<Option<AgentApplicableState>>,
}

impl AgentApplicableStateHolder {
    pub fn get_agent_applicable_state(&self) -> Option<AgentApplicableState> {
        self.agent_applicable_state.read().clone()
    }

    pub fn set_agent_applicable_state(
        &self,
        agent_applicable_state: Option<AgentApplicableState>,
    ) -> Result<()> {
        {
            let mut state = self.agent_applicable_state.write();

            (*state).clone_from(&agent_applicable_state);
        }

        Ok(self.change_notifier.send(agent_applicable_state)?)
    }

    pub fn subscribe(&self) -> watch::Receiver<Option<AgentApplicableState>> {
        self.change_notifier.subscribe()
    }
}

impl Default for AgentApplicableStateHolder {
    fn default() -> Self {
        let (change_notifier, _) = watch::channel(None);

        Self {
            agent_applicable_state: RwLock::new(None),
            change_notifier,
        }
    }
}
