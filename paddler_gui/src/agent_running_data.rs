use paddler_types::agent_controller_snapshot::AgentControllerSnapshot;
use paddler_types::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;

pub struct AgentRunningData {
    pub cluster_address: String,
    pub connected: bool,
    pub snapshot: AgentControllerSnapshot,
}

impl AgentRunningData {
    pub fn apply_status(&mut self, status: SlotAggregatedStatusSnapshot) {
        self.connected = true;
        self.snapshot = AgentControllerSnapshot {
            desired_slots_total: status.desired_slots_total,
            download_current: status.download_current,
            download_filename: status.download_filename,
            download_total: status.download_total,
            id: String::new(),
            issues: status.issues,
            model_path: status.model_path,
            name: self.snapshot.name.clone(),
            slots_processing: status.slots_processing,
            slots_total: status.slots_total,
            state_application_status: status.state_application_status,
            uses_chat_template_override: status.uses_chat_template_override,
        };
    }
}
