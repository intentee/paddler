use crate::agent_applicable_state::AgentApplicableState;

pub enum AgentDesiredStateConversion {
    Cancelled,
    Converted(AgentApplicableState),
}
