use paddler_messaging::agent_controller_snapshot::AgentControllerSnapshot;
use paddler_messaging::agent_decisions_serving::AgentDecisionsServing;
use paddler_messaging::agent_runtime_status::AgentRuntimeStatus;

#[must_use]
pub fn agent_serves_decisions(
    AgentControllerSnapshot { status, .. }: &AgentControllerSnapshot,
    decisions: AgentDecisionsServing,
) -> bool {
    matches!(
        status.runtime,
        AgentRuntimeStatus::Serving {
            decisions: served_decisions,
            ..
        } if served_decisions == decisions
    )
}
