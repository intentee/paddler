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
            .send_if_modified(|current_agent_applicable_state| {
                if *current_agent_applicable_state == agent_applicable_state {
                    return false;
                }

                *current_agent_applicable_state = agent_applicable_state;

                true
            });
    }

    #[must_use]
    pub fn subscribe(&self) -> watch::Receiver<AgentApplicableState> {
        let mut agent_applicable_state_rx = self.agent_applicable_state_tx.subscribe();

        agent_applicable_state_rx.mark_changed();

        agent_applicable_state_rx
    }
}
