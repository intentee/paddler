use paddler_types::agent_controller_snapshot::AgentControllerSnapshot;
use paddler_types::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;

pub struct AgentRunningData {
    pub balancer_address: String,
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

#[cfg(test)]
mod tests {
    #![expect(
        clippy::unnecessary_wraps,
        reason = "tests use Result<()> uniformly so the ? operator can be added without churn"
    )]

    use std::collections::BTreeSet;

    use anyhow::Result;
    use paddler_types::agent_controller_snapshot::AgentControllerSnapshot;
    use paddler_types::agent_state_application_status::AgentStateApplicationStatus;
    use paddler_types::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;

    use super::AgentRunningData;

    #[test]
    fn apply_status_marks_connected_preserves_existing_name_and_copies_snapshot_fields()
    -> Result<()> {
        let mut data = AgentRunningData {
            balancer_address: "127.0.0.1:8060".to_owned(),
            connected: false,
            snapshot: AgentControllerSnapshot {
                desired_slots_total: 0,
                download_current: 0,
                download_filename: None,
                download_total: 0,
                id: "stale-id-should-be-cleared".to_owned(),
                issues: BTreeSet::new(),
                model_path: None,
                name: Some("agent-fixture".to_owned()),
                slots_processing: 0,
                slots_total: 0,
                state_application_status: AgentStateApplicationStatus::Fresh,
                uses_chat_template_override: false,
            },
        };

        let status = SlotAggregatedStatusSnapshot {
            desired_slots_total: 6,
            download_current: 100,
            download_filename: Some("model.gguf".to_owned()),
            download_total: 200,
            issues: BTreeSet::new(),
            model_path: Some("/models/model.gguf".to_owned()),
            slots_processing: 2,
            slots_total: 6,
            state_application_status: AgentStateApplicationStatus::Applied,
            uses_chat_template_override: true,
            version: 7,
        };

        data.apply_status(status);

        assert!(data.connected, "expected connected to flip to true");
        assert_eq!(
            data.snapshot.name.as_deref(),
            Some("agent-fixture"),
            "expected existing name to be preserved"
        );
        assert!(
            data.snapshot.id.is_empty(),
            "expected id to be cleared by apply_status"
        );
        assert_eq!(
            data.snapshot.desired_slots_total, 6,
            "expected desired_slots_total to be copied from status"
        );
        assert_eq!(
            data.snapshot.slots_processing, 2,
            "expected slots_processing to be copied from status"
        );
        assert_eq!(
            data.snapshot.model_path.as_deref(),
            Some("/models/model.gguf"),
            "expected model_path to be copied from status"
        );
        assert!(
            data.snapshot.uses_chat_template_override,
            "expected uses_chat_template_override to be copied from status"
        );

        Ok(())
    }
}
