use std::sync::RwLock;

use anyhow::Result;
use tokio::sync::watch;

use crate::agent_applicable_state::AgentApplicableState;

pub struct AgentApplicableStateHolder {
    agent_applicable_state: RwLock<Option<AgentApplicableState>>,
    change_notifier: watch::Sender<Option<AgentApplicableState>>,
}

impl AgentApplicableStateHolder {
    #[expect(clippy::expect_used, reason = "mutex lock poison is unrecoverable")]
    pub fn get_agent_applicable_state(&self) -> Option<AgentApplicableState> {
        self.agent_applicable_state
            .read()
            .expect("Failed to acquire read lock")
            .clone()
    }

    #[expect(clippy::expect_used, reason = "mutex lock poison is unrecoverable")]
    pub fn set_agent_applicable_state(
        &self,
        agent_applicable_state: Option<AgentApplicableState>,
    ) -> Result<()> {
        {
            let mut state = self
                .agent_applicable_state
                .write()
                .expect("Failed to acquire write lock");

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
