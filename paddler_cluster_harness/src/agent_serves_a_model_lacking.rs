use paddler_messaging::agent_controller_snapshot::AgentControllerSnapshot;
use paddler_messaging::requested_capability::RequestedCapability;

use crate::agent_serves_a_model::agent_serves_a_model;

#[must_use]
pub const fn agent_serves_a_model_lacking(
    agent_controller_snapshot: &AgentControllerSnapshot,
    requested_capability: RequestedCapability,
) -> bool {
    agent_serves_a_model(agent_controller_snapshot)
        && !agent_controller_snapshot
            .status
            .runtime
            .serves(requested_capability)
}
