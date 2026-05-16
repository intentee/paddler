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

    use anyhow::Result;
    use anyhow::bail;
    use paddler_types::agent_controller_snapshot::AgentControllerSnapshot;
    use paddler_types::agent_state_application_status::AgentStateApplicationStatus;
    use paddler_types::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;

    use super::Action;
    use super::AgentRunningData;
    use super::Message;

    fn fresh_running_data() -> AgentRunningData {
        AgentRunningData {
            balancer_address: "127.0.0.1:8060".to_owned(),
            connected: false,
            snapshot: AgentControllerSnapshot {
                desired_slots_total: 0,
                download_current: 0,
                download_filename: None,
                download_total: 0,
                id: String::new(),
                issues: BTreeSet::new(),
                model_path: None,
                name: Some("agent-fixture".to_owned()),
                slots_processing: 0,
                slots_total: 0,
                state_application_status: AgentStateApplicationStatus::Fresh,
                uses_chat_template_override: false,
            },
        }
    }

    fn applied_status() -> SlotAggregatedStatusSnapshot {
        SlotAggregatedStatusSnapshot {
            desired_slots_total: 4,
            download_current: 0,
            download_filename: None,
            download_total: 0,
            issues: BTreeSet::new(),
            model_path: Some("/models/model.gguf".to_owned()),
            slots_processing: 1,
            slots_total: 4,
            state_application_status: AgentStateApplicationStatus::Applied,
            uses_chat_template_override: false,
            version: 1,
        }
    }

    #[test]
    fn agent_status_updated_marks_connected_and_returns_none_action() -> Result<()> {
        let mut data = fresh_running_data();

        let action = data.update(Message::AgentStatusUpdated(applied_status()));

        match action {
            Action::None => {}
            Action::Disconnect => bail!("expected Action::None"),
        }

        if !data.connected {
            bail!("expected connected flag to flip to true after status update");
        }

        Ok(())
    }

    #[test]
    fn disconnect_message_returns_disconnect_action() -> Result<()> {
        let mut data = fresh_running_data();

        match data.update(Message::Disconnect) {
            Action::Disconnect => Ok(()),
            Action::None => bail!("expected Action::Disconnect"),
        }
    }
}
