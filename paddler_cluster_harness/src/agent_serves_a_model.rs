use paddler_messaging::agent_controller_snapshot::AgentControllerSnapshot;
use paddler_messaging::agent_runtime_status::AgentRuntimeStatus;

#[must_use]
pub const fn agent_serves_a_model(
    AgentControllerSnapshot { status, .. }: &AgentControllerSnapshot,
) -> bool {
    matches!(status.runtime, AgentRuntimeStatus::Serving { .. })
}
