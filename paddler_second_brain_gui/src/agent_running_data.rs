use paddler_types::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;

pub struct AgentRunningData {
    pub agent_name: String,
    pub cluster_address: String,
    pub status: Option<SlotAggregatedStatusSnapshot>,
}
