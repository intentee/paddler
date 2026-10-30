use crate::agent_applicable_state::AgentApplicableState;

pub enum SupersedingState {
    NotSuperseded,
    SupersededBy(AgentApplicableState),
}
