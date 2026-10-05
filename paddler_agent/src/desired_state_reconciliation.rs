use paddler_messaging::agent_desired_state::AgentDesiredState;

#[derive(Debug, PartialEq)]
pub enum DesiredStateReconciliation {
    Pending(Box<AgentDesiredState>),
    Reconciled,
}
