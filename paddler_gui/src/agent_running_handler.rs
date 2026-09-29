use paddler_messaging::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;

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

    use paddler_messaging::agent_controller_snapshot::AgentControllerSnapshot;
    use paddler_messaging::agent_state_application_status::AgentStateApplicationStatus;
    use paddler_messaging::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;

    use super::Action;
    use super::Message;
    use crate::agent_running_data::AgentRunningData;

    const AGENT_NAME: &str = "gpu-box";

    fn agent_waiting_for_its_first_status() -> AgentRunningData {
        AgentRunningData {
            balancer_address: "127.0.0.1:8060".to_owned(),
            connected: false,
            snapshot: AgentControllerSnapshot {
                desired_slots_total: 0,
                download_current: 0,
                download_filename: None,
                download_indeterminate: true,
                download_total: 0,
                id: String::new(),
                issues: BTreeSet::new(),
                model_path: None,
                name: Some(AGENT_NAME.to_owned()),
                slots_processing: 0,
                slots_total: 0,
                state_application_status: AgentStateApplicationStatus::Fresh,
                uses_chat_template_override: false,
            },
        }
    }

    #[test]
    fn a_reported_status_marks_the_agent_connected_under_its_own_name() {
        let mut agent = agent_waiting_for_its_first_status();

        let action = agent.update(Message::AgentStatusUpdated(SlotAggregatedStatusSnapshot {
            desired_slots_total: 4,
            model_path: Some("/models/qwen3.gguf".to_owned()),
            slots_processing: 1,
            slots_total: 4,
            state_application_status: AgentStateApplicationStatus::Applied,
            ..SlotAggregatedStatusSnapshot::default()
        }));

        assert!(matches!(action, Action::None));
        assert!(agent.connected);
        assert_eq!(agent.snapshot.name.as_deref(), Some(AGENT_NAME));
        assert_eq!(agent.snapshot.desired_slots_total, 4);
        assert_eq!(
            agent.snapshot.model_path.as_deref(),
            Some("/models/qwen3.gguf")
        );
        assert_eq!(agent.snapshot.slots_processing, 1);
        assert_eq!(agent.snapshot.slots_total, 4);
        assert_eq!(
            agent.snapshot.state_application_status,
            AgentStateApplicationStatus::Applied
        );
    }
}
