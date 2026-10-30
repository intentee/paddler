use crate::agent_applicable_state::AgentApplicableState;
use crate::prepared_decisions::PreparedDecisions;

pub struct PreparedServing {
    pub agent_applicable_state: AgentApplicableState,
    pub prepared_decisions: PreparedDecisions,
}
