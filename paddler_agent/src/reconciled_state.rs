use paddler_messaging::desired_state_version::DesiredStateVersion;

use crate::agent_applicable_state::AgentApplicableState;

#[derive(Clone, Debug, PartialEq)]
pub struct ReconciledState {
    pub agent_applicable_state: AgentApplicableState,
    pub desired_state_version: DesiredStateVersion,
}
