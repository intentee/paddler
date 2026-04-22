use crate::running_cluster_data::RunningClusterData;
use crate::running_cluster_snapshot::RunningClusterSnapshot;

#[derive(Debug, Clone)]
pub enum Message {
    SnapshotUpdated(Box<RunningClusterSnapshot>),
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
            Message::SnapshotUpdated(snapshot) => {
                self.snapshot = *snapshot;

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
