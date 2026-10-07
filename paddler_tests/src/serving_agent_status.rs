use paddler_messaging::agent_runtime_status::AgentRuntimeStatus;
use paddler_messaging::agent_status::AgentStatus;
use paddler_messaging::inference_mode::InferenceMode;
use paddler_messaging::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;

#[must_use]
pub fn serving_agent_status(inference_mode: InferenceMode) -> SlotAggregatedStatusSnapshot {
    SlotAggregatedStatusSnapshot {
        status: AgentStatus {
            desired_slots_total: 2,
            runtime: AgentRuntimeStatus::Serving {
                inference_mode,
                slots_total: 2,
            },
            ..AgentStatus::default()
        },
        version: 1,
    }
}
