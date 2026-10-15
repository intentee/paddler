use crate::agent_applicable_state::AgentApplicableState;

#[derive(Debug, PartialEq)]
pub enum AgentDesiredStateConversion {
    Cancelled,
    Converted(AgentApplicableState),
}
