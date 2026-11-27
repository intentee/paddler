use tokio::sync::watch;

use crate::agent_applicable_state::AgentApplicableState;

#[derive(Default)]
pub struct AgentApplicableStateHolder {
    agent_applicable_state_tx: watch::Sender<AgentApplicableState>,
}

impl AgentApplicableStateHolder {
    #[must_use]
    pub fn get_agent_applicable_state(&self) -> AgentApplicableState {
        self.agent_applicable_state_tx.borrow().clone()
    }

    pub fn set_agent_applicable_state(&self, agent_applicable_state: AgentApplicableState) {
        self.agent_applicable_state_tx
            .send_replace(agent_applicable_state);
    }
}
