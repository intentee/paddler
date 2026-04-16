use paddler_types::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;

use crate::agent_running_data::AgentRunningData;

#[derive(Debug, Clone)]
pub enum Message {
    AgentStatusUpdated(SlotAggregatedStatusSnapshot),
    Disconnect,
}

pub enum Action {
    None,
    Disconnect,
}

impl AgentRunningData {
    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::AgentStatusUpdated(status) => {
                self.apply_status(status);

                Action::None
            }
            Message::Disconnect => Action::Disconnect,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use paddler_types::agent_controller_snapshot::AgentControllerSnapshot;
    use paddler_types::agent_state_application_status::AgentStateApplicationStatus;
    use paddler_types::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;

    use super::Action;
    use super::AgentRunningData;
    use super::Message;

    fn make_data() -> AgentRunningData {
        AgentRunningData {
            cluster_address: "127.0.0.1:8060".to_owned(),
            connected: false,
            snapshot: AgentControllerSnapshot {
                desired_slots_total: 0,
                download_current: 0,
                download_filename: None,
                download_total: 0,
                id: String::new(),
                issues: BTreeSet::new(),
                model_path: None,
                name: Some("before-update".to_owned()),
                slots_processing: 0,
                slots_total: 0,
                state_application_status: AgentStateApplicationStatus::Fresh,
                uses_chat_template_override: false,
            },
        }
    }

    fn make_status() -> SlotAggregatedStatusSnapshot {
        SlotAggregatedStatusSnapshot {
            desired_slots_total: 4,
            download_current: 128,
            download_filename: Some("weights.gguf".to_owned()),
            download_total: 256,
            issues: BTreeSet::new(),
            model_path: Some("/tmp/model.gguf".to_owned()),
            slots_processing: 1,
            slots_total: 4,
            state_application_status: AgentStateApplicationStatus::Applied,
            uses_chat_template_override: false,
            version: 0,
        }
    }

    #[test]
    fn status_update_sets_connected_and_returns_none_action() {
        let mut data = make_data();

        let action = data.update(Message::AgentStatusUpdated(make_status()));

        assert!(matches!(action, Action::None));
        assert!(data.connected);
    }

    #[test]
    fn status_update_applies_status_fields_and_preserves_name() {
        let mut data = make_data();

        data.update(Message::AgentStatusUpdated(make_status()));

        assert_eq!(data.snapshot.desired_slots_total, 4);
        assert_eq!(data.snapshot.download_current, 128);
        assert_eq!(data.snapshot.download_total, 256);
        assert_eq!(data.snapshot.slots_processing, 1);
        assert_eq!(data.snapshot.slots_total, 4);
        assert_eq!(data.snapshot.model_path.as_deref(), Some("/tmp/model.gguf"));
        assert_eq!(data.snapshot.name.as_deref(), Some("before-update"));
    }

    #[test]
    fn disconnect_returns_disconnect_action() {
        let mut data = make_data();

        let action = data.update(Message::Disconnect);

        assert!(matches!(action, Action::Disconnect));
    }
}
