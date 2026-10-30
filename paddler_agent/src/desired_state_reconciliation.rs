use paddler_messaging::agent_desired_state::AgentDesiredState;

use crate::agent_applicable_state::AgentApplicableState;

#[derive(Debug, PartialEq)]
pub enum DesiredStateReconciliation {
    Pending(Box<AgentDesiredState>),
    Reconciled(Box<AgentApplicableState>),
}
