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
