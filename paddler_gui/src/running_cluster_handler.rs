use paddler_types::agent_controller_snapshot::AgentControllerSnapshot;

use crate::running_cluster_data::RunningClusterData;

#[derive(Debug, Clone)]
pub enum Message {
    AgentSnapshotsUpdated(Vec<AgentControllerSnapshot>),
    Stop,
    CopyToClipboard(String),
}

pub enum Action {
    None,
    Stop,
    CopyToClipboard(String),
}

impl RunningClusterData {
    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::AgentSnapshotsUpdated(snapshots) => {
                self.agent_snapshots = snapshots;

                Action::None
            }
            Message::Stop => {
                self.stopping = true;

                Action::Stop
            }
            Message::CopyToClipboard(content) => Action::CopyToClipboard(content),
        }
    }
}
