use paddler_messaging::agent_desired_state::AgentDesiredState;
use paddler_messaging::desired_state_version::DesiredStateVersion;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DeliveredDesiredState {
    pub desired_state: AgentDesiredState,
    pub version: DesiredStateVersion,
}
