use crate::agent_applicable_state::AgentApplicableState;

#[derive(Clone, Debug, PartialEq)]
pub enum ReconciliationPhase {
    NotStarted,
    Reconciled(AgentApplicableState),
    Reconciling,
}
