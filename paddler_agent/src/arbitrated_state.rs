use crate::agent_applicable_state::AgentApplicableState;

pub enum ArbitratedState {
    Arbitrating(AgentApplicableState),
    NothingYet,
}

impl ArbitratedState {
    #[must_use]
    pub fn arbitrates(&self, agent_applicable_state: &AgentApplicableState) -> bool {
        match self {
            Self::Arbitrating(arbitrated_applicable_state) => {
                arbitrated_applicable_state == agent_applicable_state
            }
            Self::NothingYet => false,
        }
    }
}
