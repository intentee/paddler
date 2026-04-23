use crate::running_balancer_data::RunningBalancerData;
use crate::running_balancer_snapshot::RunningBalancerSnapshot;

#[derive(Debug, Clone)]
pub enum Message {
    SnapshotUpdated(Box<RunningBalancerSnapshot>),
    Stop,
    CopyToClipboard(String),
}

pub enum Action {
    None,
    Stop,
    CopyToClipboard(String),
}

impl RunningBalancerData {
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
