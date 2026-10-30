use crate::agent_applicable_state::AgentApplicableState;
use crate::agent_error::AgentError;
use crate::prepared_serving::PreparedServing;

pub enum PreparationOutcome {
    Cancelled,
    Failed {
        agent_applicable_state: AgentApplicableState,
        error: AgentError,
    },
    Prepared(PreparedServing),
}
